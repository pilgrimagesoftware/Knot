use super::*;
use crate::app_bootstrap::{
    PanelOpenPermissionSelector, PanelPermissionAllow, PanelPermissionDeny,
};
/// Which peer view a `WorkspaceWindow` currently shows - the dashboard is a
/// toggleable view of the same window's content, not a dialog or a
/// separate window (see `openspec/changes/dashboard-view/design.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum WorkspaceViewMode {
    #[default]
    Terminal,
    Dashboard,
}

/// Terminal pane geometry - shared by resize and mouse-position translation
/// so they agree on the same grid.
pub(crate) const TERMINAL_SIDEBAR_WIDTH: f32 = 250.;
pub(crate) const TERMINAL_HEADER_HEIGHT: f32 = 64.;

/// Which text size a detail line renders at, and therefore what size its
/// icon has to be.
///
/// The row's detail lines are not all one size - the agent type and persona
/// are `text_xs`, the status and folder are the UI font size - so an icon
/// fixed at one size reads as undersized beside half of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DetailLineSize {
    /// `text_xs`, for the type and persona lines.
    Small,
    /// The UI font size, for the status and folder lines.
    Body,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct QueuedPanelPrompt {
    text: String,
    failed: bool,
}

/// One labelled detail line on an agent row: a leading icon saying what the
/// line is, then the text.
///
/// Without the icon a line is a bare string whose meaning has to be inferred
/// from its content, which fails exactly when it matters - a status that
/// mentions a path, a folder named after a person.
///
/// `items_start`, not `items_center`: the persona line has no
/// `whitespace_nowrap`, so a name like "DevOps Troubleshooter" wraps onto a
/// second line, and a centred icon floats into the gap between the two. On a
/// single-line row the two are indistinguishable.
pub(crate) fn detail_line(
    icon: gpui_kit::assets::IconName, text: String, size: DetailLineSize, font_family: String,
    font_size: gpui_kit::Pixels, cx: &App,
) -> gpui_kit::AnyElement {
    let muted = cx.theme().muted_foreground;
    let icon = match size {
        DetailLineSize::Small => Icon::new(icon).xsmall(),
        DetailLineSize::Body => Icon::new(icon).small(),
    };
    let line = div()
        .font_family(font_family)
        .text_size(font_size)
        .text_color(muted);
    let line = match size {
        DetailLineSize::Small => line.text_xs(),
        // The status and folder lines truncate rather than wrap, which is
        // what keeps a long path from growing the row.
        DetailLineSize::Body => line.overflow_hidden().whitespace_nowrap().text_ellipsis(),
    };
    h_flex()
        .w_full()
        .min_w_0()
        .gap_1()
        .items_start()
        .child(icon.text_color(muted).flex_shrink_0())
        .child(line.min_w_0().child(text))
        .into_any_element()
}

/// Whether an agent of this type runs a terminal process of its own.
///
/// Under ACP-only launch only shell agents do; every other type reaches
/// its agent through an adapter subprocess owned by the panel session.
/// This is what scopes `agent-lifecycle`'s exit-driven removal: a shell
/// agent whose process exits is removed, an ACP agent whose adapter exits
/// is not.
pub(crate) fn runs_a_terminal_process(agent_type: &str) -> bool {
    agent_type == "shell"
}

/// The font family to actually render the terminal with: the user's
/// `terminal_font_name` setting if GPUI can actually resolve it (checked
/// against the platform's font catalog plus whatever we've embedded),
/// otherwise the embedded JetBrains Mono default. Guards against a stale or
/// otherwise-unresolvable persisted value (an old default, a font that was
/// uninstalled, a font-panel value AppKit accepts but GPUI's lookup
/// doesn't) silently falling back further to the proportional UI font.
pub(crate) fn terminal_font_family(
    settings: &knot_core::Settings, cx: &App,
) -> gpui_kit::SharedString {
    let requested = &settings.terminal_font_name;
    if cx
        .text_system()
        .all_font_names()
        .iter()
        .any(|name| name == requested)
    {
        requested.clone().into()
    } else {
        "JetBrains Mono".into()
    }
}

/// Measures the actual rendered cell size for `terminal_view`'s font/size,
/// rather than guessing - an overestimate (e.g. a fixed 18px row height for
/// a font that actually renders taller) reports more PTY rows than fit in
/// the pane, so content the running program draws near what it thinks is
/// the bottom (an input box, a status line) ends up laid out below the
/// visible container and never appears.
pub(crate) fn terminal_cell_size(
    cx: &App, font_family: gpui_kit::SharedString, font_size: gpui_kit::Pixels,
) -> (f32, f32) {
    let font_id = cx.text_system().resolve_font(&gpui_kit::font(font_family));
    let width = cx
        .text_system()
        .em_advance(font_id, font_size)
        .unwrap_or(px(8.));
    let ascent = cx.text_system().ascent(font_id, font_size);
    let descent = cx.text_system().descent(font_id, font_size);
    (
        f32::from(width).max(1.),
        f32::from(ascent + descent).max(1.),
    )
}

/// The permission-mode selector's element id - the one selector with
/// risk-tinted labels and a keyboard action of its own, so it needs naming
/// rather than matching on a literal in three places.
const PERMISSION_SELECTOR_ID: &str = "panel-permission-mode-selector";

/// How far the prompt box grows with its content before it starts
/// scrolling, collapsed and expanded. It auto-grows rather than sitting at
/// a fixed height: a fixed height fights the textarea's own layout, so a
/// second line made it scroll and jump on every keystroke instead of
/// simply getting taller.
const PANEL_INPUT_ROWS_COLLAPSED: usize = 6;
const PANEL_INPUT_ROWS_EXPANDED: usize = 20;

/// One sidebar agent row's render inputs, snapshotted out of the store
/// while its lock is held so the row closures don't need it. A struct
/// rather than the tuple this used to be, per the repo convention against
/// wide positional parameter lists.
struct AgentRow {
    id: Uuid,
    avatar: String,
    name: String,
    folder: String,
    state: knot_agents::AgentState,
    is_shell: bool,
    is_companion: bool,
    header_title: String,
    persona_name: Option<String>,
    /// The agent's coding-agent type (`claude`, `opencode`, ...), shown
    /// on the row so a one-letter avatar isn't the only clue to which
    /// agent is running there.
    agent_type: String,
    /// Whether the agent has a session. Keyed on liveness, not on
    /// activation mode: a `passive` agent that never started and a
    /// deactivated `active` one are in the same position - nothing is
    /// there - and the user needs to know which agents are live, not why
    /// each one is not.
    is_running: bool,
}

pub(crate) struct WorkspaceWindow {
    /// Last known diff stat per agent, refreshed off the render path - see
    /// `refresh_diff_stats`.
    diff_stats: Arc<Mutex<BTreeMap<Uuid, Option<knot_git::DiffStats>>>>,
    /// When each agent's diff stat was last *requested*, so the refresh
    /// runs on a cadence rather than once per render. Main-thread only.
    diff_stats_requested: BTreeMap<Uuid, std::time::Instant>,
    /// Agents whose PTY process has exited, queued by the reader thread and
    /// drained by the repaint poll - the callback runs off the main thread
    /// and cannot touch the view directly, the same hand-off
    /// `clipboard_writes` uses.
    exited_sessions: Arc<Mutex<Vec<Uuid>>>,
    /// Keeps the window-bounds observer alive for this window's lifetime.
    window_bounds_subscription: Option<gpui_kit::Subscription>,
    /// Set by a finished refresh so the repaint poll redraws the header.
    diff_stats_dirty: Arc<std::sync::atomic::AtomicBool>,
    /// Which config selector's popover is open, by element id, or `None`
    /// when none is. One shared flag used to back all three: because every
    /// selector's `on_open_change` wrote it and the permission selector
    /// read it, clicking Model or Effort opened the *permission* menu.
    open_config_selector: Option<&'static str>,
    store: Arc<Mutex<knot_agents::AgentStore>>,
    /// Agent-to-agent messages, for the unread badge and the idle-time
    /// delivery nudge (`mcp-messaging`). Shared with the MCP server, which
    /// is what writes to it.
    messages: Arc<Mutex<knot_messaging::MessageStore>>,
    /// The last message each agent has been nudged about, so an unread
    /// inbox produces one prompt rather than one per idle poll.
    nudged_messages: BTreeMap<Uuid, Uuid>,
    settings: knot_core::Settings,
    workspace_id: Uuid,
    selected_agent: Option<Uuid>,
    sessions: BTreeMap<Uuid, Arc<Mutex<TerminalSession<PtyTransport>>>>,
    panel_states: BTreeMap<Uuid, Arc<Mutex<panel_state::PanelState>>>,
    /// `TerminalSession::spawn_pty` runs `tokio::spawn` for the activity
    /// tracker; the UI thread has no tokio runtime of its own, so enter
    /// this one around each spawn (see `ensure_session`).
    runtime: tokio::runtime::Runtime,
    /// Focus target for the terminal grid pane - key events only reach
    /// `dispatch_key` while this is focused (click the pane to focus it).
    terminal_focus: gpui_kit::FocusHandle,
    /// OSC 52 clipboard-store requests, queued by `ensure_session`'s
    /// `on_grid_event` (which runs on the PTY reader thread) and drained
    /// by a polling loop onto the OS pasteboard via GPUI's main-thread
    /// clipboard API - the same background-thread-to-main-thread hand-off
    /// pattern `SettingsWindow` already uses for the native font panel.
    clipboard_writes: Arc<Mutex<Vec<String>>>,
    /// Live ACP connections for Panel-mode agents, keyed by agent id -
    /// independent of `sessions` (the terminal PTYs), per the
    /// `acp-panel-ui` "Switch to Terminal mid-turn" scenario: an entry
    /// here persists across a view-mode toggle, only stopped on restart.
    panel_sessions: BTreeMap<Uuid, Arc<Mutex<panel_session::PanelSessionSlot>>>,
    /// The lifecycle phase each panel session was in the last time the
    /// repaint poll looked, so a slot moving between phases repaints - see
    /// `panel_needs_repaint`.
    panel_phases: BTreeMap<Uuid, panel_session::PanelPhase>,
    /// One prompt-entry input per Panel-mode agent that has been viewed,
    /// created lazily. Not part of `Agent`/persistence - purely UI state.
    /// A `Textarea` (not a single-line `Input`) so the expand/collapse
    /// control can grow the same entity's visible height without losing
    /// in-progress text, rather than swapping to a second entity.
    panel_prompt_inputs: BTreeMap<Uuid, Entity<TextareaState>>,
    /// Keeps each prompt input's `PressEnter` subscription alive for the
    /// life of the entity it was created for (dropping a `Subscription`
    /// cancels it).
    panel_prompt_input_subscriptions: BTreeMap<Uuid, Subscription>,
    panel_prompt_queues: BTreeMap<Uuid, Vec<QueuedPanelPrompt>>,
    panel_prompt_results: Arc<Mutex<Vec<(Uuid, String, Result<(), String>)>>>,
    /// One virtualized conversation list per Panel-mode agent that has
    /// been viewed, created lazily - the `ListState` backing
    /// `render_panel`'s virtualization, and the target of the response
    /// action bar's scroll-to-user/scroll-to-top controls and the track
    /// toggle's auto-scroll.
    panel_lists: BTreeMap<Uuid, ListState>,
    /// The item count each `panel_lists` entry was last reconciled to, so
    /// `render_panel_pane` can `splice` only the rows that actually
    /// changed and leave off-screen rows' measured heights alone.
    panel_list_row_counts: BTreeMap<Uuid, usize>,
    working_indicator_last_repaint: std::time::Instant,
    /// Files/images attached via the input area's add-context control,
    /// pending the next send - cleared once the prompt is submitted.
    panel_pending_context: BTreeMap<Uuid, Vec<PathBuf>>,
    /// Panel-mode agent ids whose input area is expanded to the larger
    /// multi-line editing size; absence means collapsed (the default).
    panel_input_expanded: BTreeSet<Uuid>,
    view_mode: WorkspaceViewMode,
    dashboard_sort: dashboard::DashboardSort,
    new_agent_name_input: Entity<InputState>,
    new_agent_folder_input: Entity<InputState>,
    show_new_agent: bool,
    error: Option<String>,
}

impl Drop for WorkspaceWindow {
    fn drop(&mut self) {
        for session in self.sessions.values() {
            if let Ok(mut session) = session.lock() {
                let _ = session.shutdown();
            }
        }
    }
}

impl WorkspaceWindow {
    pub(crate) fn open(
        store: Arc<Mutex<knot_agents::AgentStore>>,
        messages: Arc<Mutex<knot_messaging::MessageStore>>, settings: knot_core::Settings,
        workspace_id: Uuid, cx: &mut App,
    ) {
        Self::open_with_selection(store, messages, settings, workspace_id, None, cx);
    }

    /// Like `open`, but overrides the agent that would otherwise be picked
    /// by `agent_selection_for_workspace` - used when a caller (e.g. a
    /// Command Center card) already knows which agent the user wants to
    /// land on.
    pub(crate) fn open_with_selection(
        store: Arc<Mutex<knot_agents::AgentStore>>,
        messages: Arc<Mutex<knot_messaging::MessageStore>>, settings: knot_core::Settings,
        workspace_id: Uuid, select_agent: Option<Uuid>, cx: &mut App,
    ) {
        let workspace_name = store
            .lock()
            .ok()
            .and_then(|store| {
                store
                    .workspaces()
                    .iter()
                    .find(|workspace| workspace.id == workspace_id)
                    .map(|workspace| workspace.name.clone())
            })
            .unwrap_or_else(|| "Workspace".to_string());
        let saved_bounds = store.lock().ok().and_then(|store| {
            store
                .workspaces()
                .iter()
                .find(|workspace| workspace.id == workspace_id)
                .and_then(|workspace| workspace.window_bounds)
        });
        let options = workspace_window_options(saved_bounds, cx);
        if let Err(error) = cx.open_window(options, move |window, cx| {
            // The OS window title (Mission Control, Cmd+`, Window menu)
            // is separate from the TitleBar row we draw
            // ourselves - without this it falls back to
            // the app's bundle name for every workspace
            // window.
            window.set_window_title(&workspace_name);
            // Order the new window front rather than letting it open
            // behind whatever has focus - matching what the settings
            // window already does when it reuses an open one.
            window.activate_window();
            let new_agent_name_input =
                cx.new(|cx| InputState::new(window, cx).placeholder("Agent name (optional)"));
            let new_agent_folder_input =
                cx.new(|cx| InputState::new(window, cx).placeholder("Agent folder path"));
            let selected_agent = select_agent.or_else(|| {
                store
                    .lock()
                    .ok()
                    .and_then(|store| agent_selection_for_workspace(&store, workspace_id))
            });
            let clipboard_writes = Arc::new(Mutex::new(Vec::new()));
            let exited_sessions: Arc<Mutex<Vec<Uuid>>> = Arc::new(Mutex::new(Vec::new()));
            let view = cx.new(|cx| {
                let mut window = WorkspaceWindow {
                    exited_sessions: Arc::clone(&exited_sessions),
                    window_bounds_subscription: None,
                    diff_stats: Arc::new(Mutex::new(BTreeMap::new())),
                    diff_stats_requested: BTreeMap::new(),
                    diff_stats_dirty: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    open_config_selector: None,
                    store,
                    messages,
                    nudged_messages: BTreeMap::new(),
                    settings,
                    workspace_id,
                    selected_agent,
                    sessions: BTreeMap::new(),
                    panel_states: BTreeMap::new(),
                    runtime: tokio::runtime::Runtime::new()
                        .expect("failed to start terminal session runtime"),
                    terminal_focus: cx.focus_handle(),
                    clipboard_writes: Arc::clone(&clipboard_writes),
                    panel_sessions: BTreeMap::new(),
                    panel_phases: BTreeMap::new(),
                    panel_prompt_inputs: BTreeMap::new(),
                    panel_prompt_input_subscriptions: BTreeMap::new(),
                    panel_prompt_queues: BTreeMap::new(),
                    panel_prompt_results: Arc::new(Mutex::new(Vec::new())),
                    panel_lists: BTreeMap::new(),
                    panel_list_row_counts: BTreeMap::new(),
                    working_indicator_last_repaint: std::time::Instant::now(),
                    panel_pending_context: BTreeMap::new(),
                    panel_input_expanded: BTreeSet::new(),
                    view_mode: WorkspaceViewMode::Terminal,
                    dashboard_sort: dashboard::DashboardSort::default(),
                    new_agent_name_input,
                    new_agent_folder_input,
                    show_new_agent: false,
                    error: None,
                };
                // Matches the Swift reference: every agent in the
                // workspace starts its session when the workspace
                // window opens, not
                // just the one initially selected.
                let agent_ids: Vec<Uuid> = window
                    .store
                    .lock()
                    .ok()
                    .and_then(|store| {
                        store
                            .workspaces()
                            .iter()
                            .find(|workspace| workspace.id == workspace_id)
                            .map(|workspace| workspace.agent_ids.clone())
                    })
                    .unwrap_or_default();
                if let Ok(mut store) = window.store.lock() {
                    // The workspace's `active` agents start
                    // here, and only they: `activated` is
                    // runtime-only and loads false, so the
                    // durable mode is consulted on every open.
                    store.activate_on_workspace_open(&agent_ids);
                    // An explicitly requested agent (a Command
                    // Center card) is a selection, and starts
                    // whatever its mode. The workspace's own
                    // restored selection is not, so a `passive`
                    // agent stays stopped across a relaunch.
                    if let Some(requested) = select_agent {
                        store.set_activated(requested, true);
                    }
                }
                for id in agent_ids {
                    window.ensure_session(id);
                    window.ensure_panel_session(id);
                }
                window
            });
            // Drains OSC 52 clipboard-store requests queued from the PTY
            // reader thread onto the OS pasteboard (see
            // `clipboard_writes`'s doc comment), and
            // repaints the terminal grid - the PTY reader
            // thread has no way to call `cx.notify()` itself, so without
            // this the grid only visibly updates on an unrelated UI event
            // (a keystroke, mouse move), making output look stalled after
            // e.g. pressing Enter.
            let notify_view = view.clone();
            let exited_drain = Arc::clone(&exited_sessions);
            cx.spawn(async move |cx| {
                loop {
                    cx.background_executor()
                        .timer(std::time::Duration::from_millis(33))
                        .await;
                    let texts = clipboard_writes
                        .lock()
                        .map(|mut queue| std::mem::take(&mut *queue))
                        .unwrap_or_default();
                    let exited = exited_drain
                        .lock()
                        .map(|mut queue| std::mem::take(&mut *queue))
                        .unwrap_or_default();
                    for text in texts {
                        cx.update(|app| {
                            app.write_to_clipboard(ClipboardItem::new_string(text));
                        });
                    }
                    cx.update(|app| {
                        notify_view.update(app, |view, cx| {
                            // A shell companion whose
                            // process exited has nothing
                            // left to show, so close it
                            // rather than leaving a dead
                            // pane that looks hung.
                            for id in &exited {
                                view.remove_agent(*id);
                                cx.notify();
                            }
                            // Messages arrive from
                            // the MCP server on another
                            // thread; this poll is
                            // where an agent going idle
                            // is noticed.
                            view.deliver_inbox_nudges();
                            let grid_dirty = view
                                .selected_agent
                                .and_then(|id| view.sessions.get(&id))
                                .and_then(|session| session.lock().ok()?.grid())
                                .is_some_and(|grid| grid.lock().unwrap().take_dirty());
                            let panel_dirty = view.panel_needs_repaint();
                            if grid_dirty || panel_dirty {
                                cx.notify();
                            }
                        });
                    });
                }
            })
            .detach();
            // Remember where the user puts this workspace's window.
            // The observer fires continuously through a drag, so the
            // store's setter reports whether the frame actually
            // changed and only then is anything written to disk.
            view.update(cx, |view, cx| {
                let subscription = cx.observe_window_bounds(window, move |view, window, _cx| {
                    let bounds = window.window_bounds().get_bounds();
                    let saved = knot_core::SavedWindowBounds {
                        x: bounds.origin.x.into(),
                        y: bounds.origin.y.into(),
                        width: bounds.size.width.into(),
                        height: bounds.size.height.into(),
                    };
                    let changed = view
                        .store
                        .lock()
                        .map(|mut store| store.set_workspace_window_bounds(workspace_id, saved))
                        .unwrap_or(false);
                    if changed {
                        view.persist_agents();
                    }
                });
                view.window_bounds_subscription = Some(subscription);
            });
            cx.new(|cx| Root::new(view, window, cx).bg(cx.theme().background))
        }) {
            eprintln!("failed to open workspace window: {error}");
        }
    }

    /// Spawns a PTY-backed terminal session for `id` if one is not already
    /// running - matches (and, per `terminal-rendering`'s tasks.md, replaces)
    /// `Shell::attach_session`'s pattern.
    ///
    /// Only agents that [`runs_a_terminal_process`] accepts get one, which
    /// is also what scopes `agent-lifecycle`'s exit-driven removal: the
    /// process-exit hook below is registered here and nowhere else.
    /// Starts `id`'s PTY terminal session - shell agents only. Non-shell
    /// agents launch exclusively through `ensure_panel_session`; this is a
    /// no-op for them (they have no `TerminalSession`, never did view-mode
    /// double-launch it).
    ///
    /// Also a no-op for an agent that is not activated - a `passive` agent
    /// nobody has selected yet, or one that was deactivated. That single
    /// check is the whole activation gate: every caller of this and of
    /// `ensure_panel_session` (window open, row click, dashboard card,
    /// repaint poll) inherits it without having to remember, per
    /// `agent-lifecycle`'s "Activation mode" requirement.
    fn ensure_session(&mut self, id: Uuid) {
        if self.sessions.contains_key(&id) {
            return;
        }
        let agent = {
            let store = self.store.lock().unwrap();
            store.agent(id).cloned()
        };
        let Some(agent) = agent else {
            return;
        };
        if !runs_a_terminal_process(&agent.agent_type) {
            return;
        }
        if !agent.activated {
            return;
        }
        self.panel_states
            .entry(id)
            .or_insert_with(|| Arc::new(Mutex::new(panel_state::PanelState::new())));
        let persona = self.settings.persona(id);
        let config = SessionConfig {
            settings: &self.settings,
            agent: &agent,
            persona,
            plugin_root: None,
        };
        let status_store = Arc::clone(&self.store);
        let status_sink = EventSink {
            on_status: Some(Box::new(move |event| {
                apply_terminal_status(&status_store, id, event.status);
            })),
            ..Default::default()
        };
        let title_store = Arc::clone(&self.store);
        let clipboard_writes = Arc::clone(&self.clipboard_writes);

        let last_output: Arc<Mutex<Option<std::time::Instant>>> = Arc::new(Mutex::new(None));
        let on_output_activity = Arc::clone(&last_output);

        let _runtime_guard = self.runtime.enter();
        let session = TerminalSession::<PtyTransport>::spawn_pty_with_exit(
            &config,
            status_sink,
            move |_| {
                if let Ok(mut last_output) = on_output_activity.lock() {
                    *last_output = Some(std::time::Instant::now());
                }
            },
            {
                let exited = Arc::clone(&self.exited_sessions);
                move |_status| {
                    if let Ok(mut exited) = exited.lock() {
                        exited.push(id);
                    }
                }
            },
            move |event| match event {
                knot_terminal::GridEvent::Title(title) => {
                    if let Ok(mut store) = title_store.lock() {
                        store.set_terminal_title(id, title);
                    }
                }
                knot_terminal::GridEvent::ClipboardStore(
                    knot_terminal::ClipboardType::Clipboard,
                    text,
                ) => {
                    if let Ok(mut queue) = clipboard_writes.lock() {
                        queue.push(text);
                    }
                }
                _ => {}
            },
        )
        .map(|session| (session, SessionPlan::build(&config)));
        match session {
            Ok((session, plan)) => {
                let session = Arc::new(Mutex::new(session));
                self.sessions.insert(id, Arc::clone(&session));
                // The shell needs a moment to switch the PTY out of canonical
                // (cooked) mode into its own raw-mode line editing; sending
                // the (often long) initialization command before that
                // happens hits the kernel's MAX_CANON line-length limit and
                // truncates it mid-command. A fixed delay isn't reliable
                // (shell startup time varies with the user's rc files), so
                // instead wait for the shell's own startup output (prompt
                // draw, MOTD, etc.) to go quiet - `QUIET_PERIOD` after the
                // last byte, capped by `MAX_WAIT` so a shell that never
                // stops printing doesn't block the command forever.
                const QUIET_PERIOD: std::time::Duration = std::time::Duration::from_millis(150);
                const POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(20);
                const MAX_WAIT: std::time::Duration = std::time::Duration::from_secs(3);
                std::thread::spawn(move || {
                    let start = std::time::Instant::now();
                    loop {
                        let quiet = last_output.lock().is_ok_and(|last_output| {
                            last_output
                                .is_some_and(|last_output| last_output.elapsed() >= QUIET_PERIOD)
                        });
                        if quiet || start.elapsed() >= MAX_WAIT {
                            break;
                        }
                        std::thread::sleep(POLL_INTERVAL);
                    }
                    if let Ok(mut session) = session.lock()
                        && let Err(error) = session.start(&plan)
                    {
                        eprintln!("failed to start terminal session: {error}");
                    }
                });
            }
            Err(error) => eprintln!("failed to start terminal session: {error}"),
        }
    }

    /// How stale a cached diff stat may get before the next render asks
    /// for a fresh one.
    const DIFF_STATS_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(2);

    /// Requests a fresh diff stat for the selected agent if the cached one
    /// has aged out, computing it on the runtime's blocking pool.
    ///
    /// `git diff --numstat` is a subprocess, and this used to run inline in
    /// `selected_agent_header` - so every render spawned one, and since a
    /// keystroke in the prompt box re-renders, typing ran at the speed of
    /// `git`. `knot-git` is runtime-agnostic by contract, hence
    /// `spawn_blocking` rather than an async call.
    fn refresh_diff_stats(&mut self, id: Uuid, folder: &str) {
        let fresh = self
            .diff_stats_requested
            .get(&id)
            .is_some_and(|at| at.elapsed() < Self::DIFF_STATS_MAX_AGE);
        if fresh {
            return;
        }
        self.diff_stats_requested
            .insert(id, std::time::Instant::now());
        let folder = folder.to_string();
        let cache = Arc::clone(&self.diff_stats);
        let dirty = Arc::clone(&self.diff_stats_dirty);
        let _runtime_guard = self.runtime.enter();
        self.runtime.spawn_blocking(move || {
            let stats = Repository::open(&folder).diff_stats().ok();
            if let Ok(mut cache) = cache.lock()
                && cache.insert(id, stats) != Some(stats)
            {
                dirty.store(true, std::sync::atomic::Ordering::SeqCst);
            }
        });
    }

    /// Whether the selected agent's panel needs a repaint: either its live
    /// session has new events, or its slot changed lifecycle phase since
    /// the last poll.
    ///
    /// The phase half matters because `ensure_panel_session` fills the slot
    /// from a background tokio task. `Ready` carries its own dirty flag,
    /// but `Failed` carries nothing - so before this check, a connection
    /// that failed (a missing API key, a refused handshake) left the pane
    /// showing "Connecting to agent…" indefinitely, making the connect
    /// timeout look like it had never fired when in fact the error was
    /// sitting in the slot, undrawn.
    fn panel_needs_repaint(&mut self) -> bool {
        let prompt_results = self
            .panel_prompt_results
            .lock()
            .map(|mut results| std::mem::take(&mut *results))
            .unwrap_or_default();
        let prompt_results_changed = !prompt_results.is_empty();
        for (id, text, result) in prompt_results {
            if let Some(queue) = self.panel_prompt_queues.get_mut(&id)
                && let Some(index) = queue.iter().position(|prompt| prompt.text == text)
            {
                if result.is_ok() {
                    queue.remove(index);
                } else if let Some(prompt) = queue.get_mut(index) {
                    prompt.failed = true;
                }
            }
        }
        let stats_changed = self
            .diff_stats_dirty
            .swap(false, std::sync::atomic::Ordering::SeqCst);
        let panel_states = self
            .panel_sessions
            .iter()
            .filter_map(|(id, slot)| {
                let slot = slot.lock().ok()?;
                let panel_session::PanelSessionSlot::Ready(handle) = &*slot else {
                    return None;
                };
                let state_arc = handle.state();
                let state = state_arc.lock().ok()?;
                let agent_state = if state.pending_permission.is_some() {
                    knot_agents::AgentState::Input
                } else if state.turn_active {
                    knot_agents::AgentState::Running
                } else {
                    knot_agents::AgentState::Idle
                };
                Some((*id, agent_state))
            })
            .collect::<Vec<_>>();
        if let Ok(mut store) = self.store.lock() {
            for (id, state) in panel_states {
                store.set_state(id, state);
            }
        }
        let Some(id) = self.selected_agent else {
            return stats_changed;
        };
        let Some(slot) = self.panel_sessions.get(&id) else {
            return false;
        };
        let (phase, events_arrived, turn_active) = {
            let slot = slot.lock().unwrap();
            let events_arrived = matches!(&*slot,
                                          panel_session::PanelSessionSlot::Ready(handle)
                                          if handle.take_dirty());
            let turn_active = match &*slot {
                panel_session::PanelSessionSlot::Ready(handle) => handle
                    .state()
                    .lock()
                    .map(|state| state.turn_active)
                    .unwrap_or(false),
                _ => false,
            };
            (slot.phase(), events_arrived, turn_active)
        };
        let phase_changed = self.panel_phases.insert(id, phase) != Some(phase);
        let indicator_due = turn_active
            && self.working_indicator_last_repaint.elapsed()
                >= std::time::Duration::from_millis(120);
        if indicator_due {
            self.working_indicator_last_repaint = std::time::Instant::now();
        }
        if !turn_active {
            self.drain_panel_prompt(id);
        }
        phase_changed || events_arrived || indicator_due || stats_changed || prompt_results_changed
    }

    /// Tears down a session (e.g. its agent was removed or restarted).
    /// Covers both launch paths: the PTY terminal session for shell
    /// agents, and the ACP connection for Panel-mode ones - a non-shell
    /// agent has no `sessions` entry at all, so without the panel half
    /// removing it left its adapter subprocess running.
    fn remove_session(&mut self, id: Uuid) {
        if let Some(session) = self.sessions.remove(&id)
            && let Ok(mut session) = session.lock()
        {
            let _ = session.shutdown();
        }
        self.panel_phases.remove(&id);
        // Drop the list with the session: its scroll handler captured the
        // old slot, so a reconnect must build a fresh list bound to the new
        // one (and its row count must start empty so the reconciler
        // splices the conversation back in).
        self.panel_lists.remove(&id);
        self.panel_list_row_counts.remove(&id);
        if let Some(slot) = self.panel_sessions.remove(&id) {
            let handle = match std::mem::replace(
                &mut *slot.lock().unwrap(),
                panel_session::PanelSessionSlot::connecting().0,
            ) {
                panel_session::PanelSessionSlot::Ready(handle) => Some(handle),
                _ => None,
            };
            if let Some(handle) = handle {
                let _runtime_guard = self.runtime.enter();
                self.runtime.spawn(async move { handle.stop().await });
            }
        }
    }

    /// Drops a failed connection so the next render starts a fresh one.
    ///
    /// Only reachable from the `Failed` slot, which owns no handle and no
    /// subprocess - there is nothing to shut down, just the dead slot to
    /// clear so `ensure_panel_session` stops short-circuiting on it.
    fn retry_panel_session(&mut self, id: Uuid) {
        self.panel_sessions.remove(&id);
        self.panel_phases.remove(&id);
        // The new connection gets a new slot; the old list's scroll handler
        // points at the dead one, so rebuild it.
        self.panel_lists.remove(&id);
        self.panel_list_row_counts.remove(&id);
        self.ensure_panel_session(id);
    }

    /// Removes `id` (and its companions) from the store, tears down their
    /// sessions, and persists the result.
    ///
    /// The persist is the point: `AgentStore::remove` only mutates memory,
    /// so without writing `saved_agents`/`saved_workspaces` back out the
    /// removal was undone by the next launch (or by any other window's
    /// persist), which is what "Remove Agent does nothing" looked like.
    /// Every other agent mutation (create, edit) already persists this way.
    fn remove_agent(&mut self, id: Uuid) {
        let removed = match self.store.lock() {
            Ok(mut store) => store.remove(id),
            Err(_) => return,
        };
        for removed_agent in removed {
            self.teardown_session(removed_agent.id);
            if self.selected_agent == Some(removed_agent.id) {
                self.selected_agent = None;
            }
        }
        self.persist_agents();
    }

    /// Tears `id`'s session and every piece of per-agent view state down,
    /// leaving the agent itself alone. Shared by [`Self::remove_agent`],
    /// which then drops the agent, and [`Self::deactivate_agent`], which
    /// does not - so a field added to one path cannot be missed in the
    /// other. That divergence is exactly the shape a session leak arrives in.
    fn teardown_session(&mut self, id: Uuid) {
        self.remove_session(id);
        self.panel_states.remove(&id);
        self.panel_prompt_inputs.remove(&id);
        self.panel_prompt_input_subscriptions.remove(&id);
        self.panel_prompt_queues.remove(&id);
        self.panel_lists.remove(&id);
        self.panel_list_row_counts.remove(&id);
        self.panel_pending_context.remove(&id);
        self.panel_input_expanded.remove(&id);
    }

    /// Makes `id` this window's selection and marks it activated, so the
    /// next `ensure_*` starts it. Selection is the universal "I want this
    /// one" signal: it starts a `passive` agent that has never run, and
    /// restarts a deactivated one whatever its activation mode - otherwise
    /// Deactivate would be a trap with no way back short of Restart.
    ///
    /// Deliberately not on the layout-restore path, which sets the field
    /// directly: restoring a window must not start what the user has not
    /// asked for.
    fn select_agent(&mut self, id: Uuid) {
        self.selected_agent = Some(id);
        if let Ok(mut store) = self.store.lock() {
            store.set_activated(id, true);
        }
    }

    /// Stops `id`'s session without removing the agent, per
    /// `agent-lifecycle`'s "Deactivating an agent" requirement: the agent
    /// keeps its name, folder, ordering, persona and activation mode, and
    /// its place in every workspace. Clearing `activated` is what stops the
    /// repaint poll from starting it straight back up; selecting the row
    /// sets it again, in either activation mode.
    ///
    /// Companions go first, mirroring the removal cascade - a companion has
    /// no session worth keeping once its owner's is gone.
    fn deactivate_agent(&mut self, id: Uuid) {
        let deactivated = match self.store.lock() {
            Ok(mut store) => store.deactivate(id),
            Err(_) => return,
        };
        for agent_id in deactivated {
            self.teardown_session(agent_id);
        }
    }

    /// Writes the store's current agents and workspaces back to settings.
    fn persist_agents(&mut self) {
        if let Ok(store) = self.store.lock() {
            self.settings.saved_agents =
                store.saved_agents(self.settings.restore_conversation_on_launch);
            self.settings.saved_workspaces = store.saved_workspaces();
        }
        let _ = self.settings.persist();
    }

    /// Starts a Panel-mode ACP connection for `id` if one isn't already
    /// running, per `agent-launch-command`'s "ACP launch path" requirement.
    /// Falls back silently (no session, no error) if the agent type has no
    /// registered adapter - the caller renders the terminal in that case,
    /// and likewise if the agent is not activated (see `ensure_session`).
    fn ensure_panel_session(&mut self, id: Uuid) {
        if self.panel_sessions.contains_key(&id) {
            return;
        }
        let agent = {
            let store = self.store.lock().unwrap();
            store.agent(id).cloned()
        };
        let Some(agent) = agent else {
            return;
        };
        if !agent.activated {
            return;
        }
        let request = knot_agent_launch::LaunchRequest {
            agent_type: &agent.agent_type,
            ..Default::default()
        };
        let knot_agent_launch::LaunchPlan::Adapter(adapter_config) =
            knot_agent_launch::plan_launch(&request)
        else {
            // No registered ACP adapter for this agent type - only shell
            // agents (which never reach `ensure_panel_session`) are meant
            // to fall through to the Terminal path.
            return;
        };
        let mcp_url = self
            .settings
            .mcp_server_enabled
            .then(|| knot_agent_launch::mcp_url(&self.settings));

        let (connecting, progress) = panel_session::PanelSessionSlot::connecting();
        let slot = Arc::new(Mutex::new(connecting));
        self.panel_sessions.insert(id, Arc::clone(&slot));
        let cwd = agent.folder.clone();
        let prior_session_id = agent.acp_session_id.clone();
        let registration_prompt = knot_agent_launch::acp_registration_prompt(
            agent.id,
            prior_session_id.is_some(),
            self.settings.persona(id),
        );
        let store = Arc::clone(&self.store);
        let _runtime_guard = self.runtime.enter();
        self.runtime.spawn(async move {
            let request = panel_session::ConnectRequest {
                config: &adapter_config,
                cwd: &cwd,
                prior_session_id: prior_session_id.as_deref(),
                mcp_url: mcp_url.as_deref(),
                registration_prompt,
            };
            panel_session::connect_into(&slot, request, &progress, |session_id| {
                if let Ok(mut store) = store.lock() {
                    store.set_acp_session_id(id, session_id.to_string());
                }
            })
            .await;
        });
    }

    /// Delivers the "check your inbox" prompt to any agent in this
    /// workspace with an unread message it has not been told about, per
    /// `mcp-messaging`'s idle-time delivery nudge.
    ///
    /// Driven from the repaint poll rather than from a send-time event,
    /// because the requirement also covers a message that arrived while its
    /// recipient was working: by the time that agent goes idle the event is
    /// long gone, but the unread message is still in the store to be found.
    fn deliver_inbox_nudges(&mut self) {
        if !self.settings.mcp_server_enabled {
            return;
        }
        let candidates = {
            let (Ok(store), Ok(messages)) = (self.store.lock(), self.messages.lock()) else {
                return;
            };
            let Some(workspace) = store
                .workspaces()
                .iter()
                .find(|workspace| workspace.id == self.workspace_id)
            else {
                return;
            };
            workspace
                .agent_ids
                .iter()
                .filter_map(|id| store.agent(*id))
                .filter_map(|agent| {
                    let latest = messages.latest_unread_id(agent.id);
                    let check = app_state::NudgeCheck {
                        agent_type: &agent.agent_type,
                        mcp_enabled: true,
                        latest_message: latest,
                        last_nudged: self.nudged_messages.get(&agent.id).copied(),
                        idle: agent.state == knot_agents::AgentState::Idle,
                        can_receive: self.panel_can_take_a_prompt(agent.id),
                    };
                    (app_state::should_inject_inbox_prompt(check))
                        .then(|| (agent.id, latest.expect("checked by the predicate")))
                })
                .collect::<Vec<_>>()
        };
        for (id, message_id) in candidates {
            self.send_inbox_nudge(id);
            self.nudged_messages.insert(id, message_id);
        }
    }

    /// Whether `id`'s panel session could take a prompt this instant: ready,
    /// no permission outstanding, no turn in flight. The same gate the
    /// composer uses - a nudge must not be what discovers a session is busy.
    fn panel_can_take_a_prompt(&self, id: Uuid) -> bool {
        let Some(slot) = self.panel_sessions.get(&id) else {
            return false;
        };
        let Ok(guard) = slot.lock() else {
            return false;
        };
        match &*guard {
            panel_session::PanelSessionSlot::Ready(handle) => handle
                .state()
                .lock()
                .map(|state| state.pending_permission.is_none() && !state.turn_active)
                .unwrap_or(false),
            _ => false,
        }
    }

    /// Sends the inbox prompt into `id`'s panel session, recording it in the
    /// conversation the way any other prompt is.
    fn send_inbox_nudge(&mut self, id: Uuid) {
        let Some(slot) = self.panel_sessions.get(&id) else {
            return;
        };
        let session = {
            let guard = slot.lock().unwrap();
            match &*guard {
                panel_session::PanelSessionSlot::Ready(handle) => {
                    handle.record_user_message(app_support::CHECK_INBOX_PROMPT.to_string());
                    Some((handle.session(), handle.recorder()))
                }
                _ => None,
            }
        };
        let Some((session, recorder)) = session else {
            return;
        };
        let _runtime_guard = self.runtime.enter();
        self.runtime.spawn(async move {
            if let Err(error) = session.prompt(app_support::CHECK_INBOX_PROMPT).await {
                recorder.error(format!(
                    "The inbox nudge could not be delivered: \
                                                    {error}"
                ));
                eprintln!("failed to deliver the inbox nudge: {error}");
            }
        });
    }

    /// Sends `id` its MCP registration prompt by hand, for an agent that
    /// failed to register at launch.
    ///
    /// Only Panel-mode agents can be registered this way, which is every
    /// non-shell agent under ACP-only launch - and the menu hides the item
    /// for shell agents. If the session is not `Ready` there is nowhere to
    /// send it; the pane is already showing that connection state, so this
    /// says nothing rather than stacking a second message on top of it.
    fn send_registration_prompt(&mut self, id: Uuid) {
        let Some(slot) = self.panel_sessions.get(&id) else {
            return;
        };
        let prompt = knot_agent_launch::registration_prompt(id);
        let session = {
            let guard = slot.lock().unwrap();
            match &*guard {
                panel_session::PanelSessionSlot::Ready(handle) => {
                    handle.record_user_message(prompt.clone());
                    Some((handle.session(), handle.recorder()))
                }
                _ => None,
            }
        };
        let Some((session, recorder)) = session else {
            return;
        };
        let _runtime_guard = self.runtime.enter();
        self.runtime.spawn(async move {
            if let Err(error) = session.prompt(&prompt).await {
                recorder.error(format!("The agent could not be registered: {error}"));
                eprintln!("failed to send the registration prompt: {error}");
            }
        });
    }

    /// Renders the markdown pane for `id`, which takes over the content
    /// area while the agent has a markdown file open.
    ///
    /// The `display-markdown` MCP tool and the "Markdown Files" context
    /// menu item both set that file; until this existed, both wrote state
    /// no UI ever read, so an agent calling the tool appeared to be
    /// ignored. Closing the pane clears the file but keeps the history, so
    /// the menu can bring it back.
    fn render_markdown_pane(
        &self, id: Uuid, file: &Path, cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let title = file
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| file.to_string_lossy().into_owned());
        // Read at render time rather than cached: the file is written by
        // an agent that may still be editing it, and re-reading is what
        // makes a second `display-markdown` of the same path show the new
        // content.
        let body = std::fs::read_to_string(file).unwrap_or_else(|error| {
            format!("Could not read `{}`:\n\n```\n{error}\n```", file.display())
        });
        v_flex()
            .size_full()
            .child(
                h_flex()
                    .w_full()
                    .flex_shrink_0()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .font_semibold()
                            .child(title),
                    )
                    .child(
                        Button::new("markdown-pane-close")
                            .icon(IconName::Close)
                            .ghost()
                            .small()
                            .tooltip("Close")
                            .on_click(cx.listener(move |view, _, _window, cx| {
                                if let Ok(mut store) = view.store.lock() {
                                    let _ = store.clear_markdown_panel(id);
                                }
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .id(("markdown-pane", id.as_u128() as u64))
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .min_w_0()
                    .overflow_y_scroll()
                    .p_4()
                    .child(TextView::markdown(
                        ("markdown-pane-body", id.as_u128() as u64),
                        body,
                    )),
            )
            .into_any_element()
    }

    /// The content pane for a selected agent that is not running: a
    /// `passive` agent nobody has started, or one that was deactivated.
    ///
    /// It exists because the alternative reads as a bug: an empty pane on an
    /// agent whose state dot says Idle is exactly what a hung agent looks
    /// like. Naming why it is not running, and that selecting it starts it,
    /// is the same guard the editor's activation hint gives from the other
    /// side.
    fn render_stopped_pane(&self, name: String, cx: &Context<Self>) -> gpui_kit::AnyElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_1()
            .child(
                div()
                    .text_color(cx.theme().foreground)
                    .child(format!("{name} is not running")),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Select this agent in the sidebar to start it."),
            )
            .into_any_element()
    }

    /// Renders the Panel-mode content pane for `id`: a connecting/failed
    /// placeholder, or the folded conversation plus a prompt input once
    /// the ACP session is ready. Starts the session if it isn't already
    /// running.
    fn render_panel_pane(
        &mut self, id: Uuid, window: &mut Window, cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        self.ensure_panel_session(id);
        let Some(slot) = self.panel_sessions.get(&id) else {
            return div().into_any_element();
        };
        let slot_guard = slot.lock().unwrap();
        match &*slot_guard {
            panel_session::PanelSessionSlot::Connecting(progress) => {
                let step = progress.lock().map(|step| step.label()).unwrap_or_default();
                v_flex()
                    .size_full()
                    .items_center()
                    .justify_center()
                    .gap_1()
                    .child(
                        div()
                            .text_color(rgb(0x9CA3AF))
                            .child("Connecting to agent…"),
                    )
                    .child(div().text_xs().text_color(rgb(0x6B7280)).child(step))
                    .into_any_element()
            }
            panel_session::PanelSessionSlot::Failed(message) => {
                let message = message.clone();
                drop(slot_guard);
                // A failed connect is often transient - a loaded machine,
                // an adapter slow to answer `initialize` - so offer the
                // retry rather than making the user remove and re-add the
                // agent to get another attempt.
                v_flex()
                    .size_full()
                    .p_4()
                    .gap_3()
                    .items_start()
                    .child(
                        div()
                            .text_color(rgb(0xEF4444))
                            .child(format!("Failed to connect: {message}")),
                    )
                    .child(
                        Button::new("panel-retry-connect")
                            .label("Try again")
                            .icon(gpui_kit::component::Icon::new(
                                gpui_kit::assets::IconName::RefreshCw,
                            ))
                            .primary()
                            .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                                view.retry_panel_session(id);
                                cx.notify();
                            })),
                    )
                    .into_any_element()
            }
            panel_session::PanelSessionSlot::Ready(handle) => {
                let state_arc = handle.state();
                let state = state_arc.lock().unwrap();
                let blocked = state.pending_permission.is_some();
                let session_arc = Arc::clone(slot);
                let pending = state.pending_permission.clone();
                let on_decision = move |decision: knot_acp::PermissionDecision| {
                    let Some(request) = &pending else {
                        return;
                    };
                    if let Ok(slot) = session_arc.lock()
                        && let panel_session::PanelSessionSlot::Ready(handle) = &*slot
                    {
                        handle.answer_permission(request, decision);
                    }
                };
                let track_slot = Arc::clone(slot);
                let on_toggle_track = move || {
                    if let Ok(slot) = track_slot.lock()
                        && let panel_session::PanelSessionSlot::Ready(handle) = &*slot
                    {
                        handle.toggle_tracking();
                    }
                };
                let tool_call_slot = Arc::clone(slot);
                let on_toggle_tool_call = move |tool_call_id: String| {
                    if let Ok(slot) = tool_call_slot.lock()
                        && let panel_session::PanelSessionSlot::Ready(handle) = &*slot
                    {
                        handle.toggle_tool_call(&tool_call_id);
                    }
                };
                let manual_slot = Arc::clone(slot);
                let on_manual_scroll = move || {
                    if let Ok(slot) = manual_slot.lock()
                        && let panel_session::PanelSessionSlot::Ready(handle) = &*slot
                    {
                        handle.clear_tracking();
                    }
                };
                let follow_slot = Arc::clone(slot);
                let list_slot = Arc::clone(slot);
                let should_follow = state.turn_active && state.tracking;
                let turn_active = state.turn_active;
                let config_options = state.config_options.clone();
                let permission_risk = Self::find_config_option(
                    &config_options,
                    &["mode", "permission_mode", "permission-mode"],
                )
                .and_then(|option| option.current_value.as_str())
                .map(|value| panel_view::permission_risk_level(value, "Permission"))
                .unwrap_or(panel_view::RiskLevel::Neutral);
                let theme = cx.theme();
                let panel_style = panel_view::PanelStyle {
                    permission_risk,
                    markdown_font_size: px(self.settings.markdown_font_size as f32),
                    mono_font_family: theme.mono_font_family.clone(),
                    ui_font_family: theme.font_family.clone(),
                    danger_color: theme.danger,
                    info_color: theme.info,
                    border_color: theme.border,
                };
                drop(state);
                drop(slot_guard);
                // Reconcile the virtualized list with the folded state:
                // splice only the rows that changed, then mirror the track
                // toggle's follow state onto the list. `Tail` while the
                // in-flight response is tracked; `Normal` otherwise, so a
                // toggled-off response stays put *even at the tail* - the
                // whole reason for hand-rolling on `ListState` rather than
                // using a scroller whose follow mode re-engages itself.
                let list = self.panel_list(id, list_slot);
                let known = self.panel_list_row_counts.get(&id).copied().unwrap_or(0);
                {
                    let state = state_arc.lock().unwrap();
                    let count = panel_view::sync_row_count(&list, known, &state);
                    self.panel_list_row_counts.insert(id, count);
                }
                if should_follow {
                    if !list.is_following_tail() {
                        list.set_follow_mode(FollowMode::Tail);
                    }
                } else {
                    list.set_follow_mode(FollowMode::Normal);
                }
                let pending_context = self
                    .panel_pending_context
                    .get(&id)
                    .cloned()
                    .unwrap_or_default();
                let queued_prompts = self
                    .panel_prompt_queues
                    .get(&id)
                    .cloned()
                    .unwrap_or_default();
                let expanded = self.panel_input_expanded.contains(&id);
                let input = self.panel_prompt_input(id, window, cx);
                // A conversation shorter than its viewport is not
                // scrollable, so `is_scrolled_to_end` is `None` and the
                // control stays hidden.
                let scrolled_up = matches!(list.is_scrolled_to_end(), Some(false));
                let list_to_bottom = list.clone();
                v_flex()
                    .size_full()
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .min_h_0()
                            .child(panel_view::render_panel(
                                Arc::clone(&state_arc),
                                list.clone(),
                                &panel_style,
                                panel_view::PanelCallbacks::new(
                                    on_decision,
                                    on_toggle_track,
                                    on_toggle_tool_call,
                                    on_manual_scroll,
                                ),
                            ))
                            .children(scrolled_up.then(|| {
                                div().absolute().bottom_3().right_4().child(
                                    Button::new("panel-scroll-to-bottom")
                                        .icon(IconName::ChevronDown)
                                        .tooltip("Scroll to latest")
                                        .small()
                                        .on_click(move |_: &ClickEvent, _, _| {
                                            list_to_bottom.scroll_to_end();
                                            // Jumping to the end also
                                            // resumes following new
                                            // output, which is what the
                                            // control implies.
                                            if let Ok(slot) = follow_slot.lock()
                                                && let panel_session::PanelSessionSlot::Ready(
                                                    handle,
                                                ) = &*slot
                                            {
                                                handle.set_tracking(true);
                                            }
                                        }),
                                )
                            })),
                    )
                    .child(self.render_panel_input_area(
                        id,
                        &input,
                        &pending_context,
                        &queued_prompts,
                        expanded,
                        blocked,
                        turn_active,
                        &config_options,
                        cx,
                    ))
                    .into_any_element()
            }
        }
    }

    /// The input area: attached-context chips, the expandable text entry,
    /// and a control row (add-context, permission mode, model, effort,
    /// expand/collapse, send) - a sibling of the message list under
    /// `render_panel_pane`, per design decision "Control bar placement".
    #[allow(clippy::too_many_arguments)]
    fn render_panel_input_area(
        &mut self, id: Uuid, input: &Entity<TextareaState>, pending_context: &[PathBuf],
        queued_prompts: &[QueuedPanelPrompt], expanded: bool, blocked: bool, turn_active: bool,
        config_options: &[knot_acp::ConfigOption], cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let can_send = !blocked && !turn_active && !input.read(cx).value().trim().is_empty();
        let shift_to_send = self.settings.agent_panel_shift_enter_sends;
        let send_tooltip = if shift_to_send {
            "Send (Shift+Enter)"
        } else {
            "Send (Enter)"
        };
        v_flex()
            .flex_shrink_0()
            .gap_2()
            .p_2()
            .border_t_1()
            .border_color(cx.theme().border)
            .children((!queued_prompts.is_empty()).then(|| {
                v_flex()
                    .gap_1()
                    .children(queued_prompts.iter().enumerate().map(|(index, prompt)| {
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child(div().flex_1().text_xs().child(prompt.text.clone()))
                            .child(div().text_xs().child(if prompt.failed {
                                "failed"
                            } else {
                                "queued"
                            }))
                            .child(
                                Button::new(("panel-queued-prompt-action", index as u64))
                                    .label(if prompt.failed { "Retry" } else { "Remove" })
                                    .ghost()
                                    .small()
                                    .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                                        if let Some(queue) = view.panel_prompt_queues.get_mut(&id)
                                            && index < queue.len()
                                        {
                                            if queue[index].failed {
                                                queue[index].failed = false;
                                            } else {
                                                queue.remove(index);
                                            }
                                            cx.notify();
                                        }
                                    })),
                            )
                    }))
            }))
            // Files and images dragged from Finder attach the same way the
            // paperclip and a pasted screenshot do, per `acp-panel-ui`'s
            // attached-context requirement.
            .drag_over::<gpui_kit::ExternalPaths>(|style, _, _, app| style.bg(app.theme().accent))
            .on_drop(
                cx.listener(move |view, paths: &gpui_kit::ExternalPaths, _, cx| {
                    view.panel_pending_context
                        .entry(id)
                        .or_default()
                        .extend(paths.paths().iter().cloned());
                    cx.notify();
                }),
            )
            .children((!pending_context.is_empty()).then(|| {
                h_flex()
                    .gap_1()
                    .flex_wrap()
                    .children(pending_context.iter().enumerate().map(|(index, path)| {
                        let file_name = path
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| path.to_string_lossy().into_owned());
                        h_flex()
                            .gap_1()
                            .items_center()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .bg(cx.theme().secondary)
                            .child(div().text_xs().child(file_name))
                            .child(
                                Button::new(("panel-remove-context", index as u64))
                                    .icon(IconName::CircleX)
                                    .ghost()
                                    .xsmall()
                                    .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                                        view.remove_panel_context(id, index);
                                        cx.notify();
                                    })),
                            )
                    }))
            }))
            // Attach, prompt, and Send share one row (`items_center`, so
            // the two buttons sit centred against the prompt box however
            // tall it is); the send hint shares the row below with the
            // config selectors, pushed apart by `justify_between`.
            .child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .gap_2()
                    .items_center()
                    .child(
                        Button::new("panel-add-context")
                            .icon(gpui_kit::component::Icon::new(
                                gpui_kit::assets::IconName::Paperclip,
                            ))
                            .tooltip("Attach files or images")
                            .ghost()
                            .small()
                            .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                                view.add_panel_context(id, cx);
                            })),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .capture_action::<Paste>({
                                let entity = cx.entity();
                                move |_, _, app| {
                                    entity.update(app, |view, cx| {
                                        if view.paste_clipboard_image_context(id, cx) {
                                            cx.stop_propagation();
                                            cx.notify();
                                        }
                                    });
                                }
                            })
                            .child(Textarea::new(input).w_full().disabled(blocked)),
                    )
                    .child(
                        Button::new("panel-send-prompt")
                            .label("Send")
                            .tooltip(send_tooltip)
                            .primary()
                            .flex_shrink_0()
                            .disabled(!can_send)
                            .on_click(cx.listener(move |view, _: &ClickEvent, window, cx| {
                                view.send_panel_prompt(id, window, cx);
                            })),
                    ),
            )
            .child(
                h_flex()
                    .w_full()
                    .min_w_0()
                    .gap_2()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_xs()
                            .font_family(self.settings.ui_font_name.clone())
                            .text_color(cx.theme().muted_foreground)
                            .child(Self::panel_prompt_send_hint(shift_to_send)),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child(self.render_panel_config_selector(
                                id,
                                PERMISSION_SELECTOR_ID,
                                "Permission",
                                "This agent doesn't report permission modes",
                                Self::find_config_option(
                                    config_options,
                                    &["mode", "permission_mode", "permission-mode"],
                                ),
                                cx,
                            ))
                            .child(self.render_panel_config_selector(
                                id,
                                "panel-model-selector",
                                "Model",
                                "This agent doesn't report selectable models",
                                Self::find_config_option(config_options, &["model"]),
                                cx,
                            ))
                            .child(self.render_panel_config_selector(
                                id,
                                "panel-effort-selector",
                                "Effort",
                                "This agent doesn't report selectable effort levels",
                                Self::find_config_option(
                                    config_options,
                                    &[
                                        "effort",
                                        "reasoning",
                                        "reasoning_effort",
                                        "reasoning-effort",
                                        "thought_level",
                                        "thought-level",
                                    ],
                                ),
                                cx,
                            ))
                            .child(
                                Button::new("panel-expand-input")
                                    .icon(if expanded {
                                        IconName::Minimize
                                    } else {
                                        IconName::Maximize
                                    })
                                    .tooltip(if expanded { "Collapse" } else { "Expand" })
                                    .ghost()
                                    .small()
                                    .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                                        view.toggle_panel_input_expanded(id, cx);
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
    }

    /// One of the input area's three selector slots (permission mode,
    /// model, effort), sourced from the agent's Session Config Options -
    /// per ACP's stabilized mechanism, a live control that applies the
    /// selection via `session/set_config_option`. Disabled with an
    /// explanatory tooltip when the agent hasn't declared a matching
    /// option (adapters vary in which axes they expose).
    fn render_panel_config_selector(
        &self, id: Uuid, element_id: &'static str, placeholder: &'static str,
        disabled_tooltip: &'static str, option: Option<&knot_acp::ConfigOption>,
        cx: &mut Context<Self>,
    ) -> gpui_kit::AnyElement {
        let Some(option) = option else {
            return Button::new(element_id)
                .label(placeholder)
                .tooltip(disabled_tooltip)
                .ghost()
                .small()
                .disabled(true)
                .into_any_element();
        };
        let current_value = option.current_value.as_str().unwrap_or_default();
        let current_label = option
            .options
            .iter()
            .find(|value| value.value == current_value)
            .map(|value| value.name.clone())
            .unwrap_or_else(|| option.name.clone());
        let config_id = option.id.clone();
        let values = option.options.clone();
        let entity = cx.entity();
        let session_arc = self.panel_sessions.get(&id).cloned();
        let is_permission_selector = element_id == PERMISSION_SELECTOR_ID;
        let selector_color = is_permission_selector
            .then(|| {
                panel_view::risk_color(panel_view::permission_risk_level(
                    current_value,
                    &option.name,
                ))
            })
            .flatten();
        let trigger = Button::new(element_id)
            .label(current_label)
            .ghost()
            .small()
            .dropdown_caret(true)
            .when_some(selector_color, |button, color| {
                button.text_color(rgb(color))
            });
        Popover::new(format!("{element_id}-{id}"))
            .anchor(gpui_kit::Anchor::BottomLeft)
            .trigger(trigger)
            .open(self.open_config_selector == Some(element_id))
            .on_open_change({
                let entity = entity.clone();
                move |open, _, app| {
                    entity.update(app, |view, cx| {
                        view.open_config_selector = open.then_some(element_id);
                        cx.notify();
                    });
                }
            })
            .content(move |_, _window, _app| {
                v_flex()
                    .gap_1()
                    .p_1()
                    .children(values.iter().enumerate().map(|(index, value)| {
                        let entity = entity.clone();
                        let session_arc = session_arc.clone();
                        let config_id = config_id.clone();
                        let value_id = value.value.clone();
                        let item_color = is_permission_selector
                            .then(|| {
                                panel_view::risk_color(panel_view::permission_risk_level(
                                    &value.value,
                                    &value.name,
                                ))
                            })
                            .flatten();
                        Button::new((element_id, index))
                            .accessibility_label(value.name.clone())
                            .child(div().w_full().child(value.name.clone()))
                            .ghost()
                            .small()
                            .w_full()
                            .when_some(item_color, |button, color| button.text_color(rgb(color)))
                            .on_click(move |_, _, app| {
                                let Some(session_arc) = session_arc.clone() else {
                                    return;
                                };
                                let config_id = config_id.clone();
                                let value_id = value_id.clone();
                                entity.update(app, move |view, _cx| {
                                    view.open_config_selector = None;
                                    if let Ok(slot) = session_arc.lock()
                                        && let panel_session::PanelSessionSlot::Ready(handle) =
                                            &*slot
                                    {
                                        let future = handle.set_config_option(config_id, value_id);
                                        let _guard = view.runtime.enter();
                                        view.runtime.spawn(future);
                                    }
                                });
                            })
                    }))
            })
            .into_any_element()
    }

    /// Finds the declared config option matching one of `categories`
    /// (case-insensitive), for bucketing the agent's arbitrary option list
    /// into the input area's three fixed selector slots.
    pub(crate) fn find_config_option<'a>(
        options: &'a [knot_acp::ConfigOption], categories: &[&str],
    ) -> Option<&'a knot_acp::ConfigOption> {
        options.iter().find(|option| {
            option.kind == "select"
                && option.category.as_deref().is_some_and(|category| {
                    categories
                        .iter()
                        .any(|candidate| candidate.eq_ignore_ascii_case(category))
                })
        })
    }

    /// Gets or creates the conversation's virtualized list for `id`'s panel.
    ///
    /// `slot` is the panel session slot the list's scroll handler clears
    /// tracking through: a user scroll (wheel or scrollbar drag) away from
    /// the tail fires the handler, which drops the in-flight response's
    /// auto-scroll - per the track toggle's "detect user-initiated scroll
    /// away from bottom" scenario - without a window/cx in the closure.
    /// The list is created empty; the caller reconciles its item count
    /// each frame (see `panel_view::sync_row_count`).
    fn panel_list(
        &mut self, id: Uuid, slot: Arc<Mutex<panel_session::PanelSessionSlot>>,
    ) -> ListState {
        if let Some(list) = self.panel_lists.get(&id) {
            return list.clone();
        }
        let list = ListState::new(0, ListAlignment::Top, px(panel_view::LIST_OVERDRAW));
        list.set_scroll_handler(move |_event, _window, _cx| {
            if let Ok(slot) = slot.lock()
                && let panel_session::PanelSessionSlot::Ready(handle) = &*slot
            {
                handle.clear_tracking();
            }
        });
        self.panel_lists.insert(id, list.clone());
        self.panel_list_row_counts.insert(id, 0);
        list
    }

    /// Opens the native file/image picker and attaches the chosen paths to
    /// `id`'s pending message, per `knot-ui-conventions`' native-picker
    /// rule (`cx.prompt_for_paths` over an in-app file browser).
    fn add_panel_context(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: true,
            prompt: Some("Attach".into()),
        });
        let this = cx.entity();
        cx.spawn(async move |_this, cx| {
            let Ok(Ok(Some(paths))) = receiver.await else {
                return;
            };
            cx.update(|app| {
                this.update(app, |view, cx| {
                    view.panel_pending_context
                        .entry(id)
                        .or_default()
                        .extend(paths);
                    cx.notify();
                });
            });
        })
        .detach();
    }

    /// Reads an image off the system clipboard (e.g. a pasted screenshot)
    /// and attaches it to `id`'s pending message the same way
    /// `add_panel_context` attaches a picked file, since ACP's
    /// `session/prompt` here only carries text plus attached paths - see
    /// `render_panel_input_area`'s `capture_action::<Paste>` wiring.
    /// Returns `false` (leaving the paste to the textarea's own text-paste
    /// handling) when the clipboard holds no image.
    fn paste_clipboard_image_context(&mut self, id: Uuid, cx: &mut App) -> bool {
        let Some(item) = cx.read_from_clipboard() else {
            return false;
        };
        let mut attached = false;
        for entry in item.entries {
            let ClipboardEntry::Image(image) = entry else {
                continue;
            };
            let extension = match image.format {
                ImageFormat::Png => "png",
                ImageFormat::Jpeg => "jpg",
                ImageFormat::Webp => "webp",
                ImageFormat::Gif => "gif",
                ImageFormat::Svg => "svg",
                ImageFormat::Bmp => "bmp",
                _ => "png",
            };
            let path = std::env::temp_dir().join(format!("knot-paste-{}.{extension}", image.id));
            if std::fs::write(&path, &image.bytes).is_ok() {
                self.panel_pending_context.entry(id).or_default().push(path);
                attached = true;
            }
        }
        attached
    }

    /// Removes one attached path from `id`'s pending context by index.
    fn remove_panel_context(&mut self, id: Uuid, index: usize) {
        if let Some(paths) = self.panel_pending_context.get_mut(&id)
            && index < paths.len()
        {
            paths.remove(index);
        }
    }

    /// Toggles `id`'s input area between its default and expanded
    /// multi-line editing size.
    fn toggle_panel_input_expanded(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let expanded = if self.panel_input_expanded.remove(&id) {
            false
        } else {
            self.panel_input_expanded.insert(id);
            true
        };
        // The cap is part of the textarea's own layout mode, so expanding
        // has to update the live entity rather than just the render height.
        let max_rows = if expanded {
            PANEL_INPUT_ROWS_EXPANDED
        } else {
            PANEL_INPUT_ROWS_COLLAPSED
        };
        if let Some(input) = self.panel_prompt_inputs.get(&id).cloned() {
            cx.update_entity(&input, |state, cx| state.set_auto_grow(1, max_rows, cx));
        }
    }

    /// Resolves the selected agent's pending permission request (if any,
    /// and if it's a Panel-mode agent) and answers it with `decision` -
    /// backs the allow/deny keybindings.
    fn answer_selected_permission(&self, decision: knot_acp::PermissionDecision) {
        let Some(id) = self.selected_agent else {
            return;
        };
        if self
            .store
            .lock()
            .ok()
            .and_then(|store| store.agent(id).map(|agent| agent.view_mode))
            != Some(knot_core::ViewMode::Panel)
        {
            return;
        }
        let Some(slot) = self.panel_sessions.get(&id) else {
            return;
        };
        let Ok(slot) = slot.lock() else {
            return;
        };
        let panel_session::PanelSessionSlot::Ready(handle) = &*slot else {
            return;
        };
        let request = handle
            .state()
            .lock()
            .ok()
            .and_then(|state| state.pending_permission.clone());
        if let Some(request) = request {
            handle.answer_permission(&request, decision);
        }
    }

    /// Gets or creates the prompt input entity for `id`'s panel, wired so
    /// `Enter`/`Shift+Enter` submits per `agent_panel_shift_enter_sends`
    /// (the other chord always inserts a newline) - see
    /// `render_panel_input_area`'s Send button tooltip for the matching
    /// user-facing hint.
    fn panel_prompt_input(
        &mut self, id: Uuid, window: &mut Window, cx: &mut Context<Self>,
    ) -> Entity<TextareaState> {
        if let Some(input) = self.panel_prompt_inputs.get(&id) {
            return input.clone();
        }
        let shift_to_send = self.settings.agent_panel_shift_enter_sends;
        let placeholder = Self::panel_prompt_placeholder();
        let max_rows = if self.panel_input_expanded.contains(&id) {
            PANEL_INPUT_ROWS_EXPANDED
        } else {
            PANEL_INPUT_ROWS_COLLAPSED
        };
        let input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder(placeholder)
                .submit_on_enter(!shift_to_send)
                .auto_grow(1, max_rows)
        });
        let subscription = cx.subscribe_in(
            &input,
            window,
            move |view: &mut Self, _, event, window, cx| {
                if let InputEvent::PressEnter { shift, .. } = event
                    && *shift == shift_to_send
                {
                    view.send_panel_prompt(id, window, cx);
                }
            },
        );
        self.panel_prompt_inputs.insert(id, input.clone());
        self.panel_prompt_input_subscriptions
            .insert(id, subscription);
        input
    }

    /// The prompt textarea's placeholder - just the prompt, not the key
    /// chord (see `panel_prompt_send_hint` for that, rendered below the
    /// textarea instead of inside it).
    fn panel_prompt_placeholder() -> &'static str {
        "Send a message…"
    }

    /// The send-chord hint shown below the prompt textarea, naming the
    /// active chord per `agent_panel_shift_enter_sends`.
    fn panel_prompt_send_hint(shift_to_send: bool) -> &'static str {
        if shift_to_send {
            "Shift+Enter to send, Enter for a newline"
        } else {
            "Enter to send, Shift+Enter for a newline"
        }
    }

    /// Reads and clears `id`'s prompt input, then sends it through the
    /// live ACP session (if any and not blocked on a pending permission),
    /// per `acp-panel-ui`'s "blocking further prompt submission until
    /// answered" requirement.
    fn send_panel_prompt(&mut self, id: Uuid, window: &mut Window, cx: &mut Context<Self>) {
        let Some(input) = self.panel_prompt_inputs.get(&id).cloned() else {
            return;
        };
        let mut text = input.read(cx).value().trim().to_string();
        if text.is_empty() {
            return;
        }
        // Attached context has no dedicated ACP content-block support here
        // (`knot-acp`'s `session/prompt` only sends a single text block),
        // so each path rides along as its own line rather than inventing
        // an unverified resource-attachment wire shape.
        let context = self.panel_pending_context.remove(&id).unwrap_or_default();
        for path in &context {
            text.push_str("\n\nAttached: ");
            text.push_str(&path.to_string_lossy());
        }
        let Some(slot) = self.panel_sessions.get(&id) else {
            return;
        };
        let session = {
            let guard = slot.lock().unwrap();
            match &*guard {
                panel_session::PanelSessionSlot::Ready(handle) => {
                    let state = handle.state();
                    let state = state.lock().unwrap();
                    let ready = state.pending_permission.is_none() && !state.turn_active;
                    drop(state);
                    ready.then(|| {
                        handle.record_user_message(text.clone());
                        (handle.session(), handle.recorder())
                    })
                }
                _ => None,
            }
        };
        let Some((session, recorder)) = session else {
            self.panel_prompt_queues
                .entry(id)
                .or_default()
                .push(QueuedPanelPrompt {
                    text,
                    failed: false,
                });
            cx.update_entity(&input, |state, cx| {
                state.set_value("", window, cx);
            });
            cx.notify();
            return;
        };
        cx.update_entity(&input, |state, cx| {
            state.set_value("", window, cx);
        });
        let _runtime_guard = self.runtime.enter();
        self.runtime.spawn(async move {
            if let Err(error) = session.prompt(&text).await {
                // Shown under the prompt it belongs to, and
                // it ends the turn - an error response is all
                // the answer this prompt gets, so the
                // composer must not stay blocked waiting for
                // a `TurnEnd` that will never arrive.
                recorder.error(format!("The agent could not answer: {error}"));
                eprintln!("failed to send panel prompt: {error}");
            }
        });
        cx.notify();
    }

    fn drain_panel_prompt(&mut self, id: Uuid) {
        let Some(slot) = self.panel_sessions.get(&id) else {
            return;
        };
        let mut candidate = || -> Option<_> {
            let guard = slot.lock().ok()?;
            let panel_session::PanelSessionSlot::Ready(handle) = &*guard else {
                return None;
            };
            let state = handle.state();
            let state = state.lock().ok()?;
            if state.pending_permission.is_some() || state.turn_active {
                return None;
            }
            let queue = self.panel_prompt_queues.get_mut(&id)?;
            let prompt = queue.first_mut()?;
            if prompt.failed {
                return None;
            }
            prompt.failed = true;
            Some((handle.session(), handle.recorder(), prompt.text.clone()))
        };
        let Some((session, recorder, text)) = candidate() else {
            return;
        };
        let results = Arc::clone(&self.panel_prompt_results);
        self.runtime.spawn(async move {
            let result = session
                .prompt(&text)
                .await
                .map_err(|error| error.to_string());
            if let Err(error) = &result {
                recorder.error(format!("The agent could not answer: {error}"));
            }
            if let Ok(mut results) = results.lock() {
                results.push((id, text, result));
            }
        });
    }

    /// Resizes `id`'s session grid/PTY to match the content pane's current
    /// size, if it changed.
    fn resize_session_to_pane(&mut self, id: Uuid, window: &Window, cx: &App) {
        let Some(session) = self.sessions.get(&id) else {
            return;
        };
        let (cell_width, cell_height) = terminal_cell_size(
            cx,
            terminal_font_family(&self.settings, cx),
            px(self.settings.terminal_font_size as f32),
        );
        let viewport = window.viewport_size();
        let pane_width = (f32::from(viewport.width) - TERMINAL_SIDEBAR_WIDTH).max(cell_width);
        let pane_height = (f32::from(viewport.height) - TERMINAL_HEADER_HEIGHT).max(cell_height);
        let size = knot_terminal::GridSize {
            columns: (pane_width / cell_width) as usize,
            rows: (pane_height / cell_height) as usize,
        };

        let current = session
            .lock()
            .ok()
            .and_then(|session| session.grid())
            .map(|grid| grid.lock().unwrap().size());
        if current != Some(size)
            && let Ok(mut session) = session.lock()
        {
            let _ = session.resize(size);
        }
    }

    /// Translates a key press on the focused terminal pane into PTY input,
    /// per `terminal-input`'s spec.
    fn dispatch_key(&mut self, id: Uuid, event: &gpui_kit::KeyDownEvent, cx: &mut App) {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform && keystroke.key == "c" {
            self.copy_selection(id, cx);
            return;
        }
        if keystroke.modifiers.platform {
            return;
        }
        let Some(session) = self.sessions.get(&id) else {
            return;
        };
        let input = knot_terminal::KeyInput {
            key: &keystroke.key,
            key_char: keystroke.key_char.as_deref(),
            control: keystroke.modifiers.control,
            alt: keystroke.modifiers.alt,
        };
        let Some(bytes) = knot_terminal::key_to_bytes(input) else {
            return;
        };
        let Ok(text) = String::from_utf8(bytes) else {
            return;
        };
        if let Ok(mut session) = session.lock() {
            let _ = session.send_text(&text);
        }
    }

    /// Converts a window-relative pixel position to a 0-indexed grid
    /// column/row, using the same pane geometry as `resize_session_to_pane`.
    fn grid_position(
        &self, position: gpui_kit::Point<gpui_kit::Pixels>, cx: &App,
    ) -> (usize, usize) {
        let (cell_width, cell_height) = terminal_cell_size(
            cx,
            terminal_font_family(&self.settings, cx),
            px(self.settings.terminal_font_size as f32),
        );
        let x = (f32::from(position.x) - TERMINAL_SIDEBAR_WIDTH).max(0.);
        let y = (f32::from(position.y) - TERMINAL_HEADER_HEIGHT).max(0.);
        ((x / cell_width) as usize, (y / cell_height) as usize)
    }

    /// Sends a mouse button press/release to the focused terminal pane's
    /// session, if the running program has enabled SGR mouse reporting -
    /// otherwise a no-op (falls back to no interaction rather than a
    /// scrollback/selection view, which isn't implemented yet).
    fn dispatch_mouse_button(
        &mut self, id: Uuid, position: gpui_kit::Point<gpui_kit::Pixels>,
        button: knot_terminal::MouseButton, pressed: bool, cx: &App,
    ) {
        let Some(session) = self.sessions.get(&id) else {
            return;
        };
        let Some(grid) = session.lock().ok().and_then(|session| session.grid()) else {
            return;
        };
        let (column, row) = self.grid_position(position, cx);
        let sgr = grid.lock().unwrap().sgr_mouse_mode();
        if !sgr {
            // No mouse-aware program is listening - left-button press starts
            // (replacing any prior) text selection instead of forwarding the
            // click, per terminal-input's spec.
            if button == knot_terminal::MouseButton::Left && pressed {
                grid.lock().unwrap().start_selection(column, row);
            }
            return;
        }
        let Some(bytes) = knot_terminal::mouse_to_bytes(
            knot_terminal::MouseInput {
                row,
                column,
                button,
                pressed,
            },
            sgr,
        ) else {
            return;
        };
        if let (Ok(text), Ok(mut session)) = (String::from_utf8(bytes), session.lock()) {
            let _ = session.send_text(&text);
        }
    }

    /// Extends an in-progress text selection while the mouse is dragged
    /// with the left button held, when no mouse-aware program has claimed
    /// mouse reporting.
    fn dispatch_mouse_drag(
        &mut self, id: Uuid, position: gpui_kit::Point<gpui_kit::Pixels>, cx: &App,
    ) {
        let Some(session) = self.sessions.get(&id) else {
            return;
        };
        let Some(grid) = session.lock().ok().and_then(|session| session.grid()) else {
            return;
        };
        let mut grid = grid.lock().unwrap();
        if grid.sgr_mouse_mode() {
            return;
        }
        let (column, row) = self.grid_position(position, cx);
        grid.update_selection(column, row);
    }

    /// Copies the focused terminal pane's active selection to the OS
    /// pasteboard, applying the terminal-actions spec's default transform
    /// (trim trailing whitespace per line).
    fn copy_selection(&mut self, id: Uuid, cx: &mut App) {
        let Some(session) = self.sessions.get(&id) else {
            return;
        };
        let Some(text) = session
            .lock()
            .ok()
            .and_then(|session| session.grid())
            .and_then(|grid| grid.lock().unwrap().selection_text())
        else {
            return;
        };
        let text: String = text
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n");
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }

    /// Sends a scroll-wheel event to the focused terminal pane's session
    /// when the running program has enabled SGR mouse reporting.
    fn dispatch_scroll(
        &mut self, id: Uuid, position: gpui_kit::Point<gpui_kit::Pixels>, lines: f32, cx: &App,
    ) {
        if lines == 0. {
            return;
        }
        let button = if lines > 0. {
            knot_terminal::MouseButton::WheelUp
        } else {
            knot_terminal::MouseButton::WheelDown
        };
        self.dispatch_mouse_button(id, position, button, true, cx);
    }

    fn create_agent(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let folder = self
            .new_agent_folder_input
            .read(cx)
            .value()
            .trim()
            .to_string();
        if folder.is_empty() || !PathBuf::from(&folder).is_dir() {
            self.error = Some("Choose an existing agent folder.".to_string());
            cx.notify();
            return false;
        }
        let name = self
            .new_agent_name_input
            .read(cx)
            .value()
            .trim()
            .to_string();
        let id = {
            let mut store = self.store.lock().unwrap();
            store.set_current_workspace(self.workspace_id);
            store.create(
                folder,
                knot_agents::CreateOptions {
                    name: (!name.is_empty()).then_some(name),
                    ..Default::default()
                },
            )
        };
        if let Ok(store) = self.store.lock() {
            self.settings.saved_agents =
                store.saved_agents(self.settings.restore_conversation_on_launch);
            self.settings.saved_workspaces = store.saved_workspaces();
        }
        let _ = self.settings.persist();
        self.select_agent(id);
        self.show_new_agent = false;
        self.error = None;
        cx.update_entity(&self.new_agent_name_input, |input, input_cx| {
            input.clean(window, input_cx);
        });
        cx.update_entity(&self.new_agent_folder_input, |input, input_cx| {
            input.clean(window, input_cx);
        });
        cx.notify();
        true
    }

    fn open_new_agent_dialog(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        open_agent_editor(
            Arc::clone(&self.store),
            self.settings.clone(),
            AgentEditorRequest {
                workspace_id: self.workspace_id,
                prefill: AgentPrefill::default(),
                insert_after: None,
                edit_target: None,
            },
            Self::select_and_focus_created_agent(cx),
            cx,
        );
    }

    /// An `on_created` callback for [`open_agent_editor`] that selects the
    /// new agent (and switches out of the dashboard, if it was open) so it
    /// becomes the visible agent in the sidebar and content pane, matching
    /// how tapping an existing agent already behaves.
    fn select_and_focus_created_agent(
        cx: &mut Context<Self>,
    ) -> impl Fn(Uuid, &mut Window, &mut App) + 'static {
        let weak = cx.entity().downgrade();
        move |id, _window, app| {
            if let Some(entity) = weak.upgrade() {
                entity.update(app, |view, cx| {
                    view.select_agent(id);
                    view.view_mode = WorkspaceViewMode::Terminal;
                    view.ensure_session(id);
                    view.ensure_panel_session(id);
                    cx.notify();
                });
            }
        }
    }
}

/// Parameters for [`open_agent_editor`], grouped to keep the function's
/// argument count in check.
pub(crate) struct SelectedAgentHeader {
    avatar: String,
    name: String,
    folder: String,
    header_title: String,
    agent_type: String,
    /// The agent's state and its diff stat *lookup*: the outer `Option`
    /// is whether the first refresh has finished, the inner one whether
    /// it found a repository. Only a missing lookup means "still
    /// working" - a folder that is not a git checkout must not sit on
    /// "Getting stats…" for the life of the window.
    state: Option<(knot_agents::AgentState, Option<Option<knot_git::DiffStats>>)>,
}

impl WorkspaceWindow {
    /// The selected agent's diff stat, with only the figures colored -
    /// additions green, deletions red, the changed-file count blue - and
    /// the words around them left muted. The count's noun goes through
    /// `l10n::plural_noun` rather than a local `if count == 1`, so the
    /// word (and its form) comes from the locale catalog.
    fn render_diff_stats(
        stats: &knot_git::DiffStats, font_family: String, font_size: gpui_kit::Pixels,
        cx: &Context<Self>,
    ) -> gpui_kit::AnyElement {
        app_state::diff_stats_row(stats, cx.theme().muted_foreground)
            .font_family(font_family)
            .text_size(font_size)
            .into_any_element()
    }

    fn selected_agent_header(&self) -> Option<SelectedAgentHeader> {
        let id = self.selected_agent?;
        let store = self.store.lock().ok()?;
        let agent = store.agent(id)?;
        let stats = self
            .diff_stats
            .lock()
            .ok()
            .and_then(|stats| stats.get(&id).copied());
        let state = (!agent.is_shell()).then_some((agent.state, stats));
        Some(SelectedAgentHeader {
            avatar: agent.avatar.clone(),
            name: agent.name.clone(),
            folder: shorten_path(&agent.folder),
            header_title: agent.header_title().to_string(),
            agent_type: agent.agent_type.clone(),
            state,
        })
    }
}

impl Render for WorkspaceWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Manrope applies explicitly to header/cell text that isn't the
        // agent's name - the name (a "title") keeps the app-wide default
        // font (Adamina), so it needs no override here.
        let ui_font_name = self.settings.ui_font_name.clone();
        let ui_font_size = px(self.settings.ui_font_size as f32);
        let (_workspace_name, agents) = {
            let store = self.store.lock().unwrap();
            let Some(workspace) = store
                .workspaces()
                .iter()
                .find(|workspace| workspace.id == self.workspace_id)
            else {
                return v_flex()
                    .size_full()
                    .child(TitleBar::new().border_color(gpui_kit::transparent_black()))
                    .child("Workspace no longer exists.");
            };
            let agents = workspace
                .agent_ids
                .iter()
                .filter_map(|id| store.agent(*id))
                .map(|agent| {
                    let persona_name = agent.persona_id.and_then(|id| {
                        self.settings
                            .personas
                            .iter()
                            .find(|persona| persona.id == id)
                            .map(|persona| persona.name.clone())
                    });
                    AgentRow {
                        id: agent.id,
                        avatar: agent.avatar.clone(),
                        name: agent.name.clone(),
                        folder: agent.folder.clone(),
                        state: agent.state,
                        is_shell: agent.is_shell(),
                        is_companion: agent.is_companion,
                        header_title: agent.header_title().to_string(),
                        persona_name,
                        agent_type: agent.agent_type.clone(),
                        is_running: agent.activated,
                    }
                })
                .collect::<Vec<_>>();
            (workspace.name.clone(), agents)
        };

        let is_dashboard = self.view_mode == WorkspaceViewMode::Dashboard;

        if !is_dashboard && let Some(id) = self.selected_agent {
            self.resize_session_to_pane(id, window, cx);
        }
        // A map lookup and an `Instant` compare per render; the `git`
        // subprocess behind it runs at most every `DIFF_STATS_MAX_AGE`.
        if let Some(id) = self.selected_agent {
            let folder = self
                .store
                .lock()
                .ok()
                .and_then(|store| store.agent(id).map(|agent| agent.folder.clone()));
            if let Some(folder) = folder {
                self.refresh_diff_stats(id, &folder);
            }
        }

        let store_for_menu = Arc::clone(&self.store);
        let settings_for_menu = self.settings.clone();
        let workspace_id = self.workspace_id;
        let window_entity = cx.entity();

        let agent_rows = agents.into_iter().map(
            |AgentRow {
                 id,
                 avatar,
                 name,
                 folder,
                 state,
                 is_shell,
                 is_companion,
                 header_title,
                 persona_name,
                 agent_type,
                 is_running,
             }| {
                let menu_name = name.clone();
                let menu_folder = folder.clone();
                let folder_name = PathBuf::from(&folder)
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or(folder);
                // Legacy/imported data may carry more
                // than one character;
                // clamp to a single grapheme so it can't
                // overflow the tile.
                let avatar = avatar.graphemes(true).next().unwrap_or("🤖").to_string();
                let selected = self.selected_agent == Some(id);
                // A plain clickable div, not `Button` -
                // `Button`'s default
                // sizing forces a fixed height
                // regardless of content,
                // clipping this row's up to 4 lines
                // (name/persona/status/
                // folder). Same fix as the dashboard
                // cards in
                // `dashboard.rs`.
                div()
                    .id(gpui_kit::ElementId::from(format!("workspace-agent-{id}")))
                    .cursor_pointer()
                    .rounded(cx.theme().radius)
                    .p_2()
                    // A companion belongs to the agent above it, so it reads
                    // as nested: indented, with a rule down its left edge.
                    .when(is_companion, |row| {
                        row.ml_4().border_l_2().border_color(cx.theme().border)
                    })
                    .bg(if selected {
                        cx.theme().muted
                    } else {
                        cx.theme().transparent
                    })
                    // A stopped agent's row is dimmed as a whole, so a
                    // workspace of mixed agents reads at a glance. The
                    // state dot cannot carry this: its four values say what
                    // a *running* agent is doing, and none of them means
                    // "not running at all". Selection still highlights the
                    // row underneath, so the selected-but-stopped agent
                    // whose pane shows the stopped placeholder is still
                    // visibly the selected one.
                    .when(!is_running, |row| row.opacity(0.45))
                    .child(
                        h_flex()
                            .w_full()
                            .gap_3()
                            .items_start()
                            .child(
                                div()
                                    .w(px(40.))
                                    .h(px(40.))
                                    .flex_shrink_0()
                                    .overflow_hidden()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_2xl()
                                    .child(avatar),
                            )
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap_0p5()
                                    .child(
                                        div()
                                            .w_full()
                                            .min_w_0()
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .text_ellipsis()
                                            .font_semibold()
                                            .child(name),
                                    )
                                    // A companion says so where a primary
                                    // agent names its type - it has no
                                    // coding-agent type of its own, and an
                                    // unlabelled row gave no clue what it was.
                                    .children(is_companion.then(|| {
                                        detail_line(
                                            gpui_kit::assets::IconName::CornerDownRight,
                                            knot_core::l10n::t("agent.companion"),
                                            DetailLineSize::Small,
                                            ui_font_name.clone(),
                                            ui_font_size,
                                            cx,
                                        )
                                    }))
                                    // The agent type reads as one of the
                                    // row's detail lines, directly under the
                                    // name and above the persona - not
                                    // right-aligned opposite it, where it
                                    // floated away from the name it
                                    // describes and crowded the state dot.
                                    .children((!is_shell).then(|| {
                                        detail_line(
                                            SettingsWindow::agent_type_icon(&agent_type),
                                            SettingsWindow::agent_type_label(&agent_type)
                                                .to_string(),
                                            DetailLineSize::Small,
                                            ui_font_name.clone(),
                                            ui_font_size,
                                            cx,
                                        )
                                    }))
                                    .children(persona_name.map(|persona_name| {
                                        // A drawn icon, not the `👤` this line
                                        // used to carry inside its text: an
                                        // emoji keeps its own colour and size,
                                        // so it was the one thing in the
                                        // column that did not line up.
                                        detail_line(
                                            gpui_kit::assets::IconName::User,
                                            persona_name,
                                            DetailLineSize::Small,
                                            ui_font_name.clone(),
                                            ui_font_size,
                                            cx,
                                        )
                                    }))
                                    .child(detail_line(
                                        gpui_kit::assets::IconName::Activity,
                                        header_title,
                                        DetailLineSize::Body,
                                        ui_font_name.clone(),
                                        ui_font_size,
                                        cx,
                                    ))
                                    .child(detail_line(
                                        gpui_kit::assets::IconName::Folder,
                                        folder_name,
                                        DetailLineSize::Body,
                                        ui_font_name.clone(),
                                        ui_font_size,
                                        cx,
                                    )),
                            )
                            .children((!is_shell).then(|| {
                                div()
                                    .flex_shrink_0()
                                    .w(px(8.))
                                    .h(px(8.))
                                    .mt_1()
                                    .rounded_full()
                                    .bg(state_color(state))
                            })),
                    )
                    .on_click(cx.listener(move |view, _: &ClickEvent, _window, cx| {
                        view.select_agent(id);
                        // Leave the dashboard, the same way tapping an
                        // agent card does - selecting a row while the
                        // dashboard was open used to change the selection
                        // without ever showing the session.
                        view.view_mode = WorkspaceViewMode::Terminal;
                        view.ensure_session(id);
                        view.ensure_panel_session(id);
                        cx.notify();
                    }))
                    .context_menu({
                        let targets = AgentMenuTargets {
                            store: Arc::clone(&store_for_menu),
                            settings: settings_for_menu.clone(),
                            window_entity: window_entity.clone(),
                            workspace_id,
                            id,
                            name: menu_name.clone(),
                            folder: menu_folder.clone(),
                        };
                        move |menu, window, cx| agent_row_context_menu(&targets, menu, window, cx)
                    })
            },
        );

        // The dashboard sits at the top of the agent list, the way the
        // Swift reference's `overviewRow` does (`SidebarView.swift`), and
        // not as an icon in the bottom bar: it is the workspace's overview
        // of every agent, so it belongs above them, shaped like the rows it
        // summarises. As a bare icon beside "New agent" it read as a minor
        // control and went unnoticed.
        let dashboard_row = div()
            .id("workspace-dashboard-row")
            .cursor_pointer()
            .rounded(cx.theme().radius)
            .p_2()
            .bg(if is_dashboard {
                cx.theme().muted
            } else {
                cx.theme().transparent
            })
            .child(
                h_flex()
                    .w_full()
                    .gap_3()
                    .items_center()
                    .child(
                        div()
                            .w(px(40.))
                            .h(px(40.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Icon::default().path("icons/layout-dashboard.svg")),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .font_semibold()
                            .child(knot_core::l10n::t("dashboard.title")),
                    ),
            )
            .on_click(cx.listener(|view, _: &ClickEvent, _window, cx| {
                view.view_mode = match view.view_mode {
                    WorkspaceViewMode::Dashboard => WorkspaceViewMode::Terminal,
                    _ => WorkspaceViewMode::Dashboard,
                };
                cx.notify();
            }));

        let selected_header = self.selected_agent_header();

        let dashboard_workspace = is_dashboard.then(|| {
            let store = self.store.lock().unwrap();
            let workspace = store
                .workspaces()
                .iter()
                .find(|workspace| workspace.id == self.workspace_id);
            let (name, color_hex, dash_agents) = match workspace {
                Some(workspace) => {
                    let dash_agents = workspace
                        .agent_ids
                        .iter()
                        .filter_map(|id| store.agent(*id))
                        .filter(|agent| !agent.is_companion)
                        .map(|agent| {
                            let folder_name = PathBuf::from(&agent.folder)
                                .file_name()
                                .map(|name| name.to_string_lossy().into_owned())
                                .unwrap_or_else(|| agent.folder.clone());
                            let git_stats = Repository::open(&agent.folder).diff_stats().ok();
                            dashboard::DashboardAgent {
                                id: agent.id,
                                avatar: agent
                                    .avatar
                                    .graphemes(true)
                                    .next()
                                    .unwrap_or("🤖")
                                    .to_string(),
                                name: agent.name.clone(),
                                folder_name,
                                state: agent.state,
                                is_shell: agent.is_shell(),
                                header_title: agent.header_title().to_string(),
                                git_stats,
                            }
                        })
                        .collect::<Vec<_>>();
                    (
                        workspace.name.clone(),
                        workspace.color_hex.clone(),
                        dash_agents,
                    )
                }
                None => (String::new(), "#1B4FB2".to_string(), Vec::new()),
            };
            dashboard::DashboardWorkspace {
                id: self.workspace_id,
                name,
                color_hex,
                agents: self.dashboard_sort.sorted(dash_agents),
            }
        });

        let weak = cx.entity().downgrade();

        let dashboard_content = dashboard_workspace.map(|dashboard_workspace| {
            let on_agent_tap = {
                let weak = weak.clone();
                move |id: Uuid, _window: &mut Window, app: &mut gpui_kit::App| {
                    if let Some(entity) = weak.upgrade() {
                        entity.update(app, |view, cx| {
                            view.select_agent(id);
                            view.view_mode = WorkspaceViewMode::Terminal;
                            view.ensure_session(id);
                            view.ensure_panel_session(id);
                            cx.notify();
                        });
                    }
                }
            };
            let on_workspace_nav = |_id: Uuid, _window: &mut Window, _app: &mut gpui_kit::App| {};
            let on_add_agent = {
                let weak = weak.clone();
                let store = Arc::clone(&self.store);
                move |workspace_id: Uuid, _window: &mut Window, app: &mut gpui_kit::App| {
                    let (folder, insert_after) = store
                        .lock()
                        .ok()
                        .and_then(|store| {
                            store
                                .workspaces()
                                .iter()
                                .find(|workspace| workspace.id == workspace_id)
                                .map(|workspace| {
                                    let folder = workspace
                                        .agent_ids
                                        .iter()
                                        .filter_map(|id| store.agent(*id))
                                        .next()
                                        .map(|agent| agent.folder.clone());
                                    (folder, workspace.agent_ids.last().copied())
                                })
                        })
                        .unwrap_or((None, None));
                    if let Some(entity) = weak.upgrade() {
                        entity.update(app, |view, cx| {
                            let on_created = WorkspaceWindow::select_and_focus_created_agent(cx);
                            open_agent_editor(
                                Arc::clone(&view.store),
                                view.settings.clone(),
                                AgentEditorRequest {
                                    workspace_id,
                                    prefill: AgentPrefill {
                                        folder,
                                        ..Default::default()
                                    },
                                    insert_after,
                                    edit_target: None,
                                },
                                on_created,
                                cx,
                            );
                        });
                    }
                }
            };

            v_flex()
                .size_full()
                .child(div().size_full().p_6().overflow_hidden().child(
                    dashboard::workspace_section(
                        dashboard_workspace,
                        false,
                        cx.theme().muted_foreground,
                        on_agent_tap,
                        on_workspace_nav,
                        on_add_agent,
                    ),
                ))
                .into_any_element()
        });

        // Matches the Swift reference's title bar: it shows the selected
        // agent's identity directly (not a separate workspace-name strip
        // above a second header row) so the header abuts the traffic
        // lights with no redundant band, and a right-hand state/git-stats
        // indicator (`AgentFullHeader`) when a non-shell agent is selected.
        let title_bar_left = if is_dashboard {
            div()
                .text_lg()
                .child(knot_core::l10n::t("dashboard.title"))
                .into_any_element()
        } else {
            match &selected_header {
                Some(header) => {
                    // The avatar and name always stay whole; the folder and
                    // the agent's status line give up space and ellipsize,
                    // the status line first since it is the longest and the
                    // least identifying.
                    h_flex()
                        .flex_1()
                        .min_w_0()
                        .items_center()
                        .gap_3()
                        .child(
                            div()
                                .flex_shrink_0()
                                .text_2xl()
                                .child(header.avatar.clone()),
                        )
                        .child(
                            div()
                                .flex_shrink_0()
                                .text_lg()
                                .font_semibold()
                                .child(header.name.clone()),
                        )
                        .child(
                            div()
                                .flex_shrink(1.)
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .font_family(ui_font_name.clone())
                                .text_size(ui_font_size)
                                .text_color(cx.theme().muted_foreground)
                                .child(header.folder.clone()),
                        )
                        .when(!header.header_title.is_empty(), |row| {
                            row.child(
                                div()
                                    .flex_shrink_0()
                                    .font_family(ui_font_name.clone())
                                    .text_size(ui_font_size)
                                    .text_color(cx.theme().muted_foreground)
                                    .child("●"),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .text_ellipsis()
                                    .font_family(ui_font_name.clone())
                                    .text_size(ui_font_size)
                                    .text_color(cx.theme().muted_foreground)
                                    .child(header.header_title.clone()),
                            )
                        })
                        .into_any_element()
                }
                None => div()
                    .text_lg()
                    .text_color(cx.theme().muted_foreground)
                    .child("Choose an agent from the sidebar")
                    .into_any_element(),
            }
        };
        let title_bar_right = if is_dashboard {
            dashboard::sort_picker(self.dashboard_sort, {
                let weak = weak.clone();
                move |sort, _window, app| {
                    if let Some(entity) = weak.upgrade() {
                        entity.update(app, |view, cx| {
                            view.dashboard_sort = sort;
                            cx.notify();
                        });
                    }
                }
            })
            .into_any_element()
        } else {
            match selected_header
                .as_ref()
                .and_then(|header| header.state.as_ref())
            {
                Some((state, git_stats)) => {
                    v_flex()
                        .items_end()
                        .gap_0p5()
                        .child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .w(px(10.))
                                        .h(px(10.))
                                        .rounded_full()
                                        .bg(state_color(*state)),
                                )
                                .child(
                                    div()
                                        .font_family(ui_font_name.clone())
                                        .text_size(ui_font_size)
                                        .text_color(cx.theme().muted_foreground)
                                        .child(state_label(*state)),
                                ),
                        )
                        .child(match git_stats {
                            Some(Some(stats)) => Self::render_diff_stats(
                                stats,
                                ui_font_name.clone(),
                                ui_font_size,
                                cx,
                            ),
                            // The refresh ran and found no
                            // repository: the agent's folder
                            // isn't a git checkout, so there
                            // are no stats to wait for.
                            Some(None) => div().into_any_element(),
                            None => div()
                                .font_family(ui_font_name.clone())
                                .text_size(ui_font_size)
                                .text_color(cx.theme().muted_foreground)
                                .child(knot_core::l10n::t("git.stats_pending"))
                                .into_any_element(),
                        })
                        .into_any_element()
                }
                None => div().into_any_element(),
            }
        };
        h_flex()
            .size_full()
            .on_action(cx.listener(|view, _: &PanelPermissionAllow, _, cx| {
                view.answer_selected_permission(knot_acp::PermissionDecision::Allow);
                cx.notify();
            }))
            .on_action(cx.listener(|view, _: &PanelPermissionDeny, _, cx| {
                view.answer_selected_permission(knot_acp::PermissionDecision::Deny);
                cx.notify();
            }))
            .on_action(cx.listener(|view, _: &PanelOpenPermissionSelector, _, cx| {
                if view.selected_agent.is_some_and(|id| {
                    view.store.lock().ok().and_then(|store| {
                        store.agent(id).map(|agent| agent.view_mode)
                    }) == Some(knot_core::ViewMode::Panel)
                }) {
                    view.open_config_selector = Some(PERMISSION_SELECTOR_ID);
                    cx.notify();
                }
            }))
            .child(
                // The sidebar column owns the traffic lights (Swift's own
                // sidebar panel does the same - they sit within its width,
                // not the content pane's). The content header below is a
                // plain sibling row, not part of this TitleBar, so it
                // starts at this column's true right edge with no gutter
                // GPUI reserves inside TitleBar for the traffic lights -
                // that's what kept misaligning it with the divider below.
                v_flex()
                    .w(px(250.))
                    .h_full()
                    .flex_shrink_0()
                    .bg(cx.theme().title_bar)
                    .child(
                        TitleBar::new()
                            .h(px(window_options::WORKSPACE_TITLE_BAR_HEIGHT))
                            .border_color(gpui_kit::transparent_black())
                            .bg(cx.theme().title_bar)
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(app_titlebar_icon())
                                    .child(knot_core::l10n::t("app.name")),
                            ),
                    )
                    .child(
                        div()
                            .id("workspace-agent-list")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .child(v_flex().gap_1()
                                           .p_4()
                                           .child(dashboard_row)
                                           .children(agent_rows)),
                    )
                    .children(
                        self.error
                            .as_ref()
                            .map(|error| div().text_sm().px_4().child(error.clone())),
                    )
                    .child(
                        h_flex()
                            .flex_shrink_0()
                            .h(px(48.))
                            .w_full()
                            .items_center()
                            .px_4()
                            .gap_2()
                            .border_t_1()
                            .border_color(cx.theme().border)
                            .child(
                                Button::new("workspace-new-agent")
                                    .icon(IconName::Plus)
                                    .label("New agent")
                                    .ghost()
                                    .on_click(cx.listener(
                                        |view, _: &ClickEvent, window, cx| {
                                            view.open_new_agent_dialog(window, cx);
                                        },
                                    )),
                            )
                    ),
            )
            .child(
                // `min_w_0` so a wide panel message (a markdown table, a
                // long command line) wraps inside this column instead of
                // stretching it past the window and pushing the prompt
                // input's Send button off screen - see
                // `knot-ui-conventions.md`'s "Flex overflow" rule.
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .children((!is_dashboard).then(|| {
                        // `min_w_0` on the row and a non-shrinking right
                        // side: at a narrow window the agent's status line
                        // used to push the whole header wider than the
                        // pane, clipping the title on one edge and running
                        // the diff stat off the other.
                        h_flex()
                            .w_full()
                            .min_w_0()
                            .flex_shrink_0()
                            .h(px(64.))
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .px_5()
                            .bg(cx.theme().background)
                            .child(title_bar_left)
                            .child(
                                h_flex().flex_shrink_0()
                                        .items_center()
                                        .gap_2()
                                        .child(title_bar_right),
                            )
                    }))
                    .child(dashboard_content.unwrap_or_else(|| {
                        // `flex_1().min_h_0()`, not `size_full()`: this box
                        // is a sibling of the 64px title bar above it, so a
                        // full height makes it overflow its container by
                        // exactly that much and pushes the input area's
                        // control row and Send button below the window edge.
                        v_flex()
                            .flex_1()
                            .min_h_0()
                            .w_full()
                            .child(
                                self.selected_agent
                                            .and_then(|id| {
                                                let (is_panel_mode, markdown_file, stopped) = {
                                                    let store = self.store.lock().unwrap();
                                                    let agent = store.agent(id);
                                                    (agent.map(|agent| agent.view_mode)
                                                     == Some(knot_core::ViewMode::Panel),
                                                     agent.and_then(|agent| {
                                                              agent.markdown_file.clone()
                                                          }),
                                                     agent.filter(|agent| !agent.activated)
                                                          .map(|agent| agent.name.clone()))
                                                };
                                                // Ahead of both session
                                                // panes: an open markdown
                                                // file takes the content
                                                // area, whichever mode the
                                                // agent otherwise runs in.
                                                if let Some(file) = markdown_file {
                                                    return Some(self.render_markdown_pane(id,
                                                                                          &file,
                                                                                          cx));
                                                }
                                                // Ahead of both session
                                                // panes, which would
                                                // otherwise render empty:
                                                // there is no session, and
                                                // asking for one is what
                                                // the activation gate
                                                // refuses.
                                                if let Some(name) = stopped {
                                                    return Some(self.render_stopped_pane(name,
                                                                                         cx));
                                                }
                                                if is_panel_mode {
                                                    return Some(self.render_panel_pane(id,
                                                                                       window,
                                                                                       cx));
                                                }
                                                let grid =
                                                    self.sessions.get(&id)?.lock().ok()?.grid()?;
                                                Some(
                                                    div()
                                                        .id("terminal-pane")
                                                        .size_full()
                                                        .track_focus(&self.terminal_focus)
                                                        .on_mouse_down(
                                                            gpui_kit::MouseButton::Left,
                                                            cx.listener(
                                                                move |view,
                                                                      event: &gpui_kit::MouseDownEvent,
                                                                      window,
                                                                      cx| {
                                                                    view.terminal_focus
                                                                        .clone()
                                                                        .focus(window, cx);
                                                                    view.dispatch_mouse_button(
                                                                        id,
                                                                        event.position,
                                                                        knot_terminal::MouseButton::Left,
                                                                        true,
                                                                        cx,
                                                                    );
                                                                },
                                                            ),
                                                        )
                                                        .on_mouse_up(
                                                            gpui_kit::MouseButton::Left,
                                                            cx.listener(
                                                                move |view,
                                                                      event: &gpui_kit::MouseUpEvent,
                                                                      _window,
                                                                      cx| {
                                                                    view.dispatch_mouse_button(
                                                                        id,
                                                                        event.position,
                                                                        knot_terminal::MouseButton::Left,
                                                                        false,
                                                                        cx,
                                                                    );
                                                                },
                                                            ),
                                                        )
                                                        .on_mouse_move(cx.listener(
                                                            move |view,
                                                                  event: &gpui_kit::MouseMoveEvent,
                                                                  _window,
                                                                  cx| {
                                                                if event.dragging() {
                                                                    view.dispatch_mouse_drag(
                                                                        id,
                                                                        event.position,
                                                                        cx,
                                                                    );
                                                                }
                                                            },
                                                        ))
                                                        .on_scroll_wheel(cx.listener(
                                                            move |view,
                                                                  event: &gpui_kit::ScrollWheelEvent,
                                                                  _window,
                                                                  cx| {
                                                                let (_, cell_height) =
                                                                    terminal_cell_size(
                                                                        cx,
                                                                        terminal_font_family(
                                                                            &view.settings,
                                                                            cx,
                                                                        ),
                                                                        px(view
                                                                            .settings
                                                                            .terminal_font_size
                                                                            as f32),
                                                                    );
                                                                let lines = match event.delta {
                                                                    gpui_kit::ScrollDelta::Lines(
                                                                        point,
                                                                    ) => point.y,
                                                                    gpui_kit::ScrollDelta::Pixels(
                                                                        point,
                                                                    ) => {
                                                                        f32::from(point.y)
                                                                            / cell_height
                                                                    }
                                                                };
                                                                view.dispatch_scroll(
                                                                    id,
                                                                    event.position,
                                                                    lines,
                                                                    cx,
                                                                );
                                                            },
                                                        ))
                                                        .on_key_down(cx.listener(
                                                            move |view, event, _window, cx| {
                                                                view.dispatch_key(id, event, cx);
                                                            },
                                                        ))
                                                        .child(terminal_view::render_grid(
                                                            &grid.lock().unwrap(),
                                                            terminal_font_family(
                                                                &self.settings,
                                                                cx,
                                                            ),
                                                            px(self.settings.terminal_font_size
                                                                as f32),
                                                        ))
                                                        .into_any_element(),
                                                )
                                            })
                                            .unwrap_or_else(|| {
                                                div()
                                                    .size_full()
                                                    .p_6()
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .bg(cx.theme().muted)
                                                    .child(
                                                        div()
                                                            .text_color(cx.theme().muted_foreground)
                                                            .child(if self.selected_agent.is_some()
                                                            {
                                                                "Starting terminal…"
                                                            } else {
                                                                "Choose an agent from the sidebar"
                                                            }),
                                                    )
                                                    .into_any_element()
                                            }),
                                    )
                                    .into_any_element()
                            })),
                    )
                    .children(app_support::root_overlays(window, cx))
    }
}

/// Everything the agent-row context menu's handlers need. Grouped so the
/// builder takes one argument instead of seven, and so the row render can
/// clone it once per row rather than capturing each piece separately.
#[derive(Clone)]
pub(crate) struct AgentMenuTargets {
    store: Arc<Mutex<knot_agents::AgentStore>>,
    settings: knot_core::Settings,
    window_entity: Entity<WorkspaceWindow>,
    workspace_id: Uuid,
    id: Uuid,
    name: String,
    folder: String,
}

/// Reads an agent's menu facts, the workspaces it could move to, and its
/// markdown history out of `store`.
///
/// Pure over the store so the rules that decide the item set - detached
/// workspaces are not move targets, an agent's own workspace is not one
/// either - are testable without a window. Called when the menu opens
/// rather than when the row renders, so a menu never offers a workspace
/// that was closed since the last repaint.
pub(crate) fn agent_menu_facts(
    store: &knot_agents::AgentStore, id: Uuid,
) -> (AgentMenuFacts, Vec<(Uuid, String)>, Vec<PathBuf>) {
    let Some(agent) = store.agent(id) else {
        return (AgentMenuFacts::default(), Vec::new(), Vec::new());
    };
    let own_workspace = store
        .workspaces()
        .iter()
        .find(|workspace| workspace.agent_ids.contains(&id))
        .map(|workspace| workspace.id);
    // Detached workspaces live in their own windows and are not move
    // targets, matching the reference's `attachedWorkspaces`.
    let move_targets = store
        .workspaces()
        .iter()
        .filter(|workspace| workspace.is_detached != Some(true))
        .filter(|workspace| Some(workspace.id) != own_workspace)
        .map(|workspace| (workspace.id, workspace.name.clone()))
        .collect::<Vec<_>>();
    let history = agent.markdown_history.clone();
    let facts = AgentMenuFacts {
        is_companion: agent.is_companion,
        is_shell: agent.is_shell(),
        has_move_targets: own_workspace.is_some() && !move_targets.is_empty(),
        has_markdown_history: !history.is_empty(),
        is_running: agent.activated,
    };
    (facts, move_targets, history)
}

/// Opens the agent editor from a menu handler, notifying the workspace
/// window once the dialog is submitted.
fn open_editor_from_menu(
    targets: &AgentMenuTargets, prefill: AgentPrefill, insert_after: Option<Uuid>,
    edit_target: Option<Uuid>, app: &mut App,
) {
    let window_entity = targets.window_entity.clone();
    open_agent_editor(
        Arc::clone(&targets.store),
        targets.settings.clone(),
        AgentEditorRequest {
            workspace_id: targets.workspace_id,
            prefill,
            insert_after,
            edit_target,
        },
        move |_id, _window, app| {
            window_entity.update(app, |_, cx| cx.notify());
        },
        app,
    );
}

/// Builds the agent row's context menu: the entries
/// `agent_context_menu_entries` decides on, each with its handler.
///
/// Every dialog opens through `window.defer`. A `PopupMenu` dismisses
/// itself immediately after running a handler, and a dialog opened inline
/// goes down with it; opening on the next turn of the loop lets the menu
/// finish closing first.
pub(crate) fn agent_row_context_menu(
    targets: &AgentMenuTargets, menu: PopupMenu, window: &mut Window, cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let (facts, move_targets, markdown_history) = match targets.store.lock() {
        Ok(store) => agent_menu_facts(&store, targets.id),
        Err(_) => (AgentMenuFacts::default(), Vec::new(), Vec::new()),
    };
    let mut menu = menu;
    for entry in agent_context_menu_entries(facts) {
        menu = match entry {
            AgentMenuEntry::Separator => menu.separator(),
            AgentMenuEntry::MoveToWorkspace => {
                let targets = targets.clone();
                let move_targets = move_targets.clone();
                menu.submenu("Move to Workspace", window, cx, move |mut submenu, _, _| {
                    for (workspace_id, workspace_name) in &move_targets {
                        let targets = targets.clone();
                        let workspace_id = *workspace_id;
                        submenu =
                            submenu.item(PopupMenuItem::new(workspace_name.clone()).on_click(
                                move |_, _window, app| {
                                    if let Ok(mut store) = targets.store.lock() {
                                        store.move_to_workspace(targets.id, workspace_id);
                                    }
                                    targets.window_entity.update(app, |view, cx| {
                                        // The moved agent may have
                                        // been this window's
                                        // selection, and it no
                                        // longer belongs here.
                                        if view.selected_agent == Some(targets.id) {
                                            view.selected_agent = None;
                                        }
                                        view.persist_agents();
                                        cx.notify();
                                    });
                                },
                            ));
                    }
                    submenu
                })
            }
            AgentMenuEntry::OpenIn => {
                let folder = targets.folder.clone();
                menu.submenu("Open In…", window, cx, move |mut submenu, _, _| {
                    for item in open_in::open_in_entries() {
                        submenu = match item {
                            open_in::OpenInEntry::Separator => submenu.separator(),
                            open_in::OpenInEntry::App(app_entry) => {
                                let folder = folder.clone();
                                submenu.item(PopupMenuItem::new(app_entry.label).on_click(
                                    move |_, _window, _app| {
                                        open_in::open_folder(app_entry.id, &folder);
                                    },
                                ))
                            }
                        };
                    }
                    submenu
                })
            }
            AgentMenuEntry::MarkdownFiles => {
                let targets = targets.clone();
                let history = markdown_history.clone();
                menu.submenu("Markdown Files", window, cx, move |mut submenu, _, _| {
                    for file in &history {
                        let targets = targets.clone();
                        let file = file.clone();
                        // File name, not the full path: the reference
                        // labels these by `lastPathComponent`, and a
                        // full path makes the submenu unreadable.
                        let label = file
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| file.to_string_lossy().into_owned());
                        submenu = submenu.item(PopupMenuItem::new(label).on_click({
                            move |_, _window, app| {
                                if let Ok(mut store) = targets.store.lock() {
                                    let _ =
                                        store.set_markdown_panel(targets.id, file.clone(), false);
                                }
                                targets.window_entity.update(app, |_, cx| cx.notify());
                            }
                        }));
                    }
                    submenu
                })
            }
            entry => {
                let Some(label) = entry.label() else {
                    continue;
                };
                let targets = targets.clone();
                menu.item(PopupMenuItem::new(label).on_click(move |_, window, app| {
                    run_agent_menu_action(entry, &targets, window, app);
                }))
            }
        };
    }
    menu
}

/// Runs one plain (non-submenu) menu entry.
fn run_agent_menu_action(
    entry: AgentMenuEntry, targets: &AgentMenuTargets, window: &mut Window, app: &mut App,
) {
    match entry {
        AgentMenuEntry::EditAgent => {
            open_editor_from_menu(
                targets,
                AgentPrefill::default(),
                None,
                Some(targets.id),
                app,
            );
        }
        // The configurable companion: same shape as "New Shell Companion"
        // but routed through the editor, so the user names it and picks
        // its folder before it exists.
        AgentMenuEntry::NewCompanion => {
            let prefill = AgentPrefill {
                folder: Some(targets.folder.clone()),
                agent_type: Some("shell".to_string()),
                created_by: Some(targets.id),
                is_companion: true,
                ..Default::default()
            };
            open_editor_from_menu(targets, prefill, Some(targets.id), None, app);
        }
        AgentMenuEntry::Deactivate => {
            // No confirmation: nothing is lost that selecting the row will
            // not bring back, which is the test Restart and Remove fail.
            targets.window_entity.update(app, |view, cx| {
                view.deactivate_agent(targets.id);
                cx.notify();
            });
        }
        AgentMenuEntry::NewShellCompanion => {
            let created = targets
                .store
                .lock()
                .map(|mut store| store.create_shell_companion(targets.id).is_ok())
                .unwrap_or(false);
            if created {
                targets.window_entity.update(app, |view, cx| {
                    view.persist_agents();
                    cx.notify();
                });
            }
        }
        // A fork carries the source's session so it picks the conversation
        // up; a duplicate deliberately does not.
        AgentMenuEntry::ForkAgent => {
            let source = targets
                .store
                .lock()
                .ok()
                .and_then(|store| store.agent(targets.id).cloned());
            let Some(source) = source else {
                return;
            };
            let prefill = AgentPrefill {
                name: Some(format!("{} (fork)", source.name)),
                avatar: Some(source.avatar.clone()),
                folder: Some(source.folder.clone()),
                agent_type: Some(source.agent_type.clone()),
                persona_id: source.persona_id,
                created_by: None,
                is_companion: false,
                session_id: source.session_id.clone(),
            };
            open_editor_from_menu(targets, prefill, Some(targets.id), None, app);
        }
        AgentMenuEntry::DuplicateAgent => {
            let created = {
                let Ok(mut store) = targets.store.lock() else {
                    return;
                };
                let Some(source) = store.agent(targets.id).cloned() else {
                    return;
                };
                store.create(
                    source.folder.clone(),
                    knot_agents::CreateOptions {
                        name: Some(format!("{} (copy)", source.name)),
                        avatar: Some(source.avatar.clone()),
                        agent_type: Some(source.agent_type.clone()),
                        shell_command: source.shell_command.clone(),
                        persona_id: source.persona_id,
                        insert_after: Some(targets.id),
                        ..Default::default()
                    },
                )
            };
            targets.window_entity.update(app, |view, cx| {
                view.persist_agents();
                view.select_agent(created);
                cx.notify();
            });
        }
        // Freshly loaded settings, not this window's snapshot: the bench
        // is edited from the settings window too, and persisting a stale
        // copy would drop whatever was added there since.
        AgentMenuEntry::SaveToBench => {
            let source = targets
                .store
                .lock()
                .ok()
                .and_then(|store| store.agent(targets.id).cloned());
            let Some(source) = source else {
                return;
            };
            let mut settings =
                knot_core::Settings::load().unwrap_or_else(|_| targets.settings.clone());
            let mut entry = knot_core::BenchAgent::new(
                Uuid::new_v4(),
                source.name.clone(),
                Some(source.avatar.clone()),
                source.folder.clone(),
            );
            entry.agent_type = source.agent_type.clone();
            entry.shell_command = source.shell_command.clone();
            entry.persona_id = source.persona_id;
            if let Err(error) = settings.add_bench_agent(entry) {
                eprintln!("failed to save the agent to the bench: {error}");
            }
        }
        AgentMenuEntry::RegisterAgent => {
            targets.window_entity.update(app, |view, cx| {
                view.send_registration_prompt(targets.id);
                cx.notify();
            });
        }
        AgentMenuEntry::RestartAgent => {
            let targets = targets.clone();
            window.defer(app, move |window, app| {
                window.open_alert_dialog(app, move |alert, _, _| {
                    let targets = targets.clone();
                    alert
                        .title("Restart Agent")
                        .description(format!(
                            "Restart \"{}\"? Its session will be \
                                                           cleared.",
                            targets.name
                        ))
                        .confirm()
                        .on_ok(move |_, _, app| {
                            if let Ok(mut store) = targets.store.lock() {
                                let _ = store.restart(targets.id);
                            }
                            targets.window_entity.update(app, |view, cx| {
                                view.remove_session(targets.id);
                                view.panel_states.remove(&targets.id);
                                // `restart` clears the
                                // persisted session ids;
                                // write them
                                // out so a
                                // relaunch doesn't resume
                                // the session
                                // just dropped.
                                //
                                view.persist_agents();
                                cx.notify();
                            });
                            true
                        })
                });
            });
        }
        AgentMenuEntry::RemoveAgent => {
            let targets = targets.clone();
            window.defer(app, move |window, app| {
                window.open_alert_dialog(app, move |alert, _, _| {
                    let targets = targets.clone();
                    alert
                        .title("Remove Agent")
                        .description(format!(
                            "Remove \"{}\"? This closes its session \
                                                           and cannot be undone.",
                            targets.name
                        ))
                        .confirm()
                        .on_ok(move |_, _, app| {
                            targets.window_entity.update(app, |view, cx| {
                                view.remove_agent(targets.id);
                                cx.notify();
                            });
                            true
                        })
                });
            });
        }
        // Handled by the builder, which needs `Window`/`Context` to make
        // a submenu, or carries no action at all.
        AgentMenuEntry::Separator
        | AgentMenuEntry::MoveToWorkspace
        | AgentMenuEntry::OpenIn
        | AgentMenuEntry::MarkdownFiles => {}
    }
}
