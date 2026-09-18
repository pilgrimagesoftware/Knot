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

/// The font family to actually render the terminal with: the user's
/// `terminal_font_name` setting if GPUI can actually resolve it (checked
/// against the platform's font catalog plus whatever we've embedded),
/// otherwise the embedded JetBrains Mono default. Guards against a stale or
/// otherwise-unresolvable persisted value (an old default, a font that was
/// uninstalled, a font-panel value AppKit accepts but GPUI's lookup
/// doesn't) silently falling back further to the proportional UI font.
pub(crate) fn terminal_font_family(settings: &knot_core::Settings, cx: &App)
                                   -> gpui_kit::SharedString {
    let requested = &settings.terminal_font_name;
    if cx.text_system()
         .all_font_names()
         .iter()
         .any(|name| name == requested)
    {
        requested.clone().into()
    }
    else {
        "JetBrains Mono".into()
    }
}

/// Measures the actual rendered cell size for `terminal_view`'s font/size,
/// rather than guessing - an overestimate (e.g. a fixed 18px row height for
/// a font that actually renders taller) reports more PTY rows than fit in
/// the pane, so content the running program draws near what it thinks is
/// the bottom (an input box, a status line) ends up laid out below the
/// visible container and never appears.
pub(crate) fn terminal_cell_size(cx: &App, font_family: gpui_kit::SharedString,
                                 font_size: gpui_kit::Pixels)
                                 -> (f32, f32) {
    let font_id = cx.text_system().resolve_font(&gpui_kit::font(font_family));
    let width = cx.text_system()
                  .em_advance(font_id, font_size)
                  .unwrap_or(px(8.));
    let ascent = cx.text_system().ascent(font_id, font_size);
    let descent = cx.text_system().descent(font_id, font_size);
    (f32::from(width).max(1.), f32::from(ascent + descent).max(1.))
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
/// How many pixels away from the bottom still counts as "at the bottom",
/// so the scroll-to-latest control doesn't flicker on sub-pixel offsets.
const SCROLL_BOTTOM_EPSILON: f32 = 8.;

const PANEL_INPUT_ROWS_COLLAPSED: usize = 6;
const PANEL_INPUT_ROWS_EXPANDED: usize = 20;

/// One sidebar agent row's render inputs, snapshotted out of the store
/// while its lock is held so the row closures don't need it. A struct
/// rather than the tuple this used to be, per the repo convention against
/// wide positional parameter lists.
struct AgentRow {
    id:           Uuid,
    avatar:       String,
    name:         String,
    folder:       String,
    state:        knot_agents::AgentState,
    is_shell:     bool,
    is_companion: bool,
    header_title: String,
    persona_name: Option<String>,
    /// The agent's coding-agent type (`claude`, `opencode`, ...), shown
    /// on the row so a one-letter avatar isn't the only clue to which
    /// agent is running there.
    agent_type:   String,
}

pub(crate) struct WorkspaceWindow {
    /// Last known diff stat per agent, refreshed off the render path - see
    /// `refresh_diff_stats`.
    diff_stats:                       Arc<Mutex<BTreeMap<Uuid, Option<knot_git::DiffStats>>>>,
    /// When each agent's diff stat was last *requested*, so the refresh
    /// runs on a cadence rather than once per render. Main-thread only.
    diff_stats_requested:             BTreeMap<Uuid, std::time::Instant>,
    /// Set by a finished refresh so the repaint poll redraws the header.
    diff_stats_dirty:                 Arc<std::sync::atomic::AtomicBool>,
    /// Which config selector's popover is open, by element id, or `None`
    /// when none is. One shared flag used to back all three: because every
    /// selector's `on_open_change` wrote it and the permission selector
    /// read it, clicking Model or Effort opened the *permission* menu.
    open_config_selector:             Option<&'static str>,
    store:                            Arc<Mutex<knot_agents::AgentStore>>,
    settings:                         knot_core::Settings,
    workspace_id:                     Uuid,
    selected_agent:                   Option<Uuid>,
    sessions:                         BTreeMap<Uuid, Arc<Mutex<TerminalSession<PtyTransport>>>>,
    panel_states:                     BTreeMap<Uuid, Arc<Mutex<panel_state::PanelState>>>,
    /// `TerminalSession::spawn_pty` runs `tokio::spawn` for the activity
    /// tracker; the UI thread has no tokio runtime of its own, so enter
    /// this one around each spawn (see `ensure_session`).
    runtime:                          tokio::runtime::Runtime,
    /// Focus target for the terminal grid pane - key events only reach
    /// `dispatch_key` while this is focused (click the pane to focus it).
    terminal_focus:                   gpui_kit::FocusHandle,
    /// OSC 52 clipboard-store requests, queued by `ensure_session`'s
    /// `on_grid_event` (which runs on the PTY reader thread) and drained
    /// by a polling loop onto the OS pasteboard via GPUI's main-thread
    /// clipboard API - the same background-thread-to-main-thread hand-off
    /// pattern `SettingsWindow` already uses for the native font panel.
    clipboard_writes:                 Arc<Mutex<Vec<String>>>,
    /// Live ACP connections for Panel-mode agents, keyed by agent id -
    /// independent of `sessions` (the terminal PTYs), per the
    /// `acp-panel-ui` "Switch to Terminal mid-turn" scenario: an entry
    /// here persists across a view-mode toggle, only stopped on restart.
    panel_sessions:                   BTreeMap<Uuid, Arc<Mutex<panel_session::PanelSessionSlot>>>,
    /// The lifecycle phase each panel session was in the last time the
    /// repaint poll looked, so a slot moving between phases repaints - see
    /// `panel_needs_repaint`.
    panel_phases:                     BTreeMap<Uuid, panel_session::PanelPhase>,
    /// One prompt-entry input per Panel-mode agent that has been viewed,
    /// created lazily. Not part of `Agent`/persistence - purely UI state.
    /// A `Textarea` (not a single-line `Input`) so the expand/collapse
    /// control can grow the same entity's visible height without losing
    /// in-progress text, rather than swapping to a second entity.
    panel_prompt_inputs:              BTreeMap<Uuid, Entity<TextareaState>>,
    /// Keeps each prompt input's `PressEnter` subscription alive for the
    /// life of the entity it was created for (dropping a `Subscription`
    /// cancels it).
    panel_prompt_input_subscriptions: BTreeMap<Uuid, Subscription>,
    /// One conversation scroll handle per Panel-mode agent that has been
    /// viewed, created lazily - backs the response action bar's
    /// scroll-to-user/scroll-to-top controls and the track toggle's
    /// auto-scroll.
    panel_scroll_handles:             BTreeMap<Uuid, gpui_kit::ScrollHandle>,
    /// Files/images attached via the input area's add-context control,
    /// pending the next send - cleared once the prompt is submitted.
    panel_pending_context:            BTreeMap<Uuid, Vec<PathBuf>>,
    /// Panel-mode agent ids whose input area is expanded to the larger
    /// multi-line editing size; absence means collapsed (the default).
    panel_input_expanded:             BTreeSet<Uuid>,
    view_mode:                        WorkspaceViewMode,
    dashboard_sort:                   dashboard::DashboardSort,
    new_agent_name_input:             Entity<InputState>,
    new_agent_folder_input:           Entity<InputState>,
    show_new_agent:                   bool,
    error:                            Option<String>,
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
    pub(crate) fn open(store: Arc<Mutex<knot_agents::AgentStore>>,
                       settings: knot_core::Settings, workspace_id: Uuid, cx: &mut App) {
        Self::open_with_selection(store, settings, workspace_id, None, cx);
    }

    /// Like `open`, but overrides the agent that would otherwise be picked
    /// by `agent_selection_for_workspace` - used when a caller (e.g. a
    /// Command Center card) already knows which agent the user wants to
    /// land on.
    pub(crate) fn open_with_selection(store: Arc<Mutex<knot_agents::AgentStore>>,
                                      settings: knot_core::Settings, workspace_id: Uuid,
                                      select_agent: Option<Uuid>, cx: &mut App) {
        let workspace_name = store.lock()
                                  .ok()
                                  .and_then(|store| {
                                      store.workspaces()
                                           .iter()
                                           .find(|workspace| workspace.id == workspace_id)
                                           .map(|workspace| workspace.name.clone())
                                  })
                                  .unwrap_or_else(|| "Workspace".to_string());
        let options = workspace_window_options(cx);
        if let Err(error) =
            cx.open_window(options, move |window, cx| {
                  // The OS window title (Mission Control, Cmd+`, Window menu)
                  // is separate from the TitleBar row we draw
                  // ourselves - without this it falls back to
                  // the app's bundle name for every workspace
                  // window.
                  window.set_window_title(&workspace_name);
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
                  let view =
                      cx.new(|cx| {
                            let mut window = WorkspaceWindow {
                    diff_stats: Arc::new(Mutex::new(BTreeMap::new())),
                    diff_stats_requested: BTreeMap::new(),
                    diff_stats_dirty: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                    open_config_selector: None,
                    store,
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
                    panel_scroll_handles: BTreeMap::new(),
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
                            let agent_ids: Vec<Uuid> =
                                window.store
                                      .lock()
                                      .ok()
                                      .and_then(|store| {
                                          store.workspaces()
                                               .iter()
                                               .find(|workspace| workspace.id == workspace_id)
                                               .map(|workspace| workspace.agent_ids.clone())
                                      })
                                      .unwrap_or_default();
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
                  cx.spawn(async move |cx| {
                        loop {
                            cx.background_executor()
                              .timer(std::time::Duration::from_millis(33))
                              .await;
                            let texts =
                                clipboard_writes.lock()
                                                .map(|mut queue| std::mem::take(&mut *queue))
                                                .unwrap_or_default();
                            for text in texts {
                                cx.update(|app| {
                                      app.write_to_clipboard(ClipboardItem::new_string(text));
                                  });
                            }
                            cx.update(|app| {
                                  notify_view.update(app, |view, cx| {
                                                 let grid_dirty =
                                                     view.selected_agent
                                                         .and_then(|id| view.sessions.get(&id))
                                                         .and_then(|session| {
                                                             session.lock().ok()?.grid()
                                                         })
                                                         .is_some_and(|grid| {
                                                             grid.lock().unwrap().take_dirty()
                                                         });
                                                 let panel_dirty = view.panel_needs_repaint();
                                                 if grid_dirty || panel_dirty {
                                                     cx.notify();
                                                 }
                                             });
                              });
                        }
                    })
                    .detach();
                  cx.new(|cx| Root::new(view, window, cx).bg(cx.theme().background))
              })
        {
            eprintln!("failed to open workspace window: {error}");
        }
    }

    /// Spawns a PTY-backed terminal session for `id` if one is not already
    /// running - matches (and, per `terminal-rendering`'s tasks.md, replaces)
    /// `Shell::attach_session`'s pattern.
    /// Starts `id`'s PTY terminal session - shell agents only. Non-shell
    /// agents launch exclusively through `ensure_panel_session`; this is a
    /// no-op for them (they have no `TerminalSession`, never did view-mode
    /// double-launch it).
    fn ensure_session(&mut self, id: Uuid) {
        if self.sessions.contains_key(&id) {
            return;
        }
        let agent = {
            let store = self.store.lock().unwrap();
            store.agent(id).cloned()
        };
        let Some(agent) = agent
        else {
            return;
        };
        if agent.agent_type != "shell" {
            return;
        }
        self.panel_states
            .entry(id)
            .or_insert_with(|| Arc::new(Mutex::new(panel_state::PanelState::new())));
        let persona = self.settings.persona(id);
        let config = SessionConfig { settings: &self.settings,
                                     agent: &agent,
                                     persona,
                                     plugin_root: None };
        let status_store = Arc::clone(&self.store);
        let status_sink =
            EventSink { on_status: Some(Box::new(move |event| {
                                            apply_terminal_status(&status_store, id, event.status);
                                        })),
                        ..Default::default() };
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
            |_| {},
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
                                                          last_output.is_some_and(|last_output| {
                                                                         last_output.elapsed()
                                                                         >= QUIET_PERIOD
                                                                     })
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
        let fresh = self.diff_stats_requested
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
        let stats_changed = self.diff_stats_dirty
                                .swap(false, std::sync::atomic::Ordering::SeqCst);
        let Some(id) = self.selected_agent
        else {
            return stats_changed;
        };
        let Some(slot) = self.panel_sessions.get(&id)
        else {
            return false;
        };
        let (phase, events_arrived) = {
            let slot = slot.lock().unwrap();
            let events_arrived = matches!(&*slot,
                                          panel_session::PanelSessionSlot::Ready(handle)
                                          if handle.take_dirty());
            (slot.phase(), events_arrived)
        };
        let phase_changed = self.panel_phases.insert(id, phase) != Some(phase);
        phase_changed || events_arrived || stats_changed
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
        if let Some(slot) = self.panel_sessions.remove(&id) {
            let handle = match std::mem::replace(&mut *slot.lock().unwrap(),
                                                 panel_session::PanelSessionSlot::connecting().0)
            {
                panel_session::PanelSessionSlot::Ready(handle) => Some(handle),
                _ => None,
            };
            if let Some(handle) = handle {
                let _runtime_guard = self.runtime.enter();
                self.runtime.spawn(async move { handle.stop().await });
            }
        }
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
            self.remove_session(removed_agent.id);
            self.panel_states.remove(&removed_agent.id);
            self.panel_prompt_inputs.remove(&removed_agent.id);
            self.panel_prompt_input_subscriptions
                .remove(&removed_agent.id);
            self.panel_scroll_handles.remove(&removed_agent.id);
            self.panel_pending_context.remove(&removed_agent.id);
            self.panel_input_expanded.remove(&removed_agent.id);
            if self.selected_agent == Some(removed_agent.id) {
                self.selected_agent = None;
            }
        }
        self.persist_agents();
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
    /// registered adapter - the caller renders the terminal in that case.
    fn ensure_panel_session(&mut self, id: Uuid) {
        if self.panel_sessions.contains_key(&id) {
            return;
        }
        let agent = {
            let store = self.store.lock().unwrap();
            store.agent(id).cloned()
        };
        let Some(agent) = agent
        else {
            return;
        };
        let request = knot_agent_launch::LaunchRequest { agent_type: &agent.agent_type,
                                                         ..Default::default() };
        let knot_agent_launch::LaunchPlan::Adapter(adapter_config) =
            knot_agent_launch::plan_launch(&request)
        else {
            // No registered ACP adapter for this agent type - only shell
            // agents (which never reach `ensure_panel_session`) are meant
            // to fall through to the Terminal path.
            return;
        };
        let mcp_url = self.settings
                          .mcp_server_enabled
                          .then(|| knot_agent_launch::mcp_url(&self.settings));

        let (connecting, progress) = panel_session::PanelSessionSlot::connecting();
        let slot = Arc::new(Mutex::new(connecting));
        self.panel_sessions.insert(id, Arc::clone(&slot));
        let cwd = agent.folder.clone();
        let prior_session_id = agent.acp_session_id.clone();
        let registration_prompt =
            knot_agent_launch::acp_registration_prompt(agent.id,
                                                       prior_session_id.is_some(),
                                                       self.settings.persona(id));
        let store = Arc::clone(&self.store);
        let _runtime_guard = self.runtime.enter();
        self.runtime.spawn(async move {
                        match panel_session::PanelSessionHandle::start(
                &adapter_config,
                &cwd,
                prior_session_id.as_deref(),
                mcp_url.as_deref(),
                &progress,
            )
            .await
            {
                Ok(handle) => {
                    if let Ok(mut store) = store.lock() {
                        store.set_acp_session_id(id, handle.session_id().to_string());
                    }
                    if let Some(prompt) = registration_prompt {
                        if let Ok(mut step) = progress.lock() {
                            *step = knot_terminal::ConnectStep::Registering;
                        }
                        handle.record_user_message(prompt.clone());
                        if let Err(error) = handle.prompt(&prompt).await {
                            eprintln!("failed to send panel registration prompt: {error}");
                        }
                    }
                    *slot.lock().unwrap() = panel_session::PanelSessionSlot::Ready(handle);
                }
                Err(error) => {
                    *slot.lock().unwrap() =
                        panel_session::PanelSessionSlot::Failed(error.to_string());
                }
            }
                    });
    }

    /// Renders the Panel-mode content pane for `id`: a connecting/failed
    /// placeholder, or the folded conversation plus a prompt input once
    /// the ACP session is ready. Starts the session if it isn't already
    /// running.
    fn render_panel_pane(&mut self, id: Uuid, window: &mut Window, cx: &mut Context<Self>)
                         -> gpui_kit::AnyElement {
        self.ensure_panel_session(id);
        let Some(slot) = self.panel_sessions.get(&id)
        else {
            return div().into_any_element();
        };
        let slot_guard = slot.lock().unwrap();
        match &*slot_guard {
            panel_session::PanelSessionSlot::Connecting(progress) => {
                let step = progress.lock().map(|step| step.label()).unwrap_or_default();
                v_flex().size_full()
                        .items_center()
                        .justify_center()
                        .gap_1()
                        .child(div().text_color(rgb(0x9CA3AF))
                                    .child("Connecting to agent…"))
                        .child(div().text_xs().text_color(rgb(0x6B7280)).child(step))
                        .into_any_element()
            }
            panel_session::PanelSessionSlot::Failed(message) => {
                div().size_full()
                     .p_4()
                     .text_color(rgb(0xEF4444))
                     .child(format!("Failed to connect: {message}"))
                     .into_any_element()
            }
            panel_session::PanelSessionSlot::Ready(handle) => {
                let state_arc = handle.state();
                let state = state_arc.lock().unwrap();
                let blocked = state.pending_permission.is_some();
                let session_arc = Arc::clone(slot);
                let pending = state.pending_permission.clone();
                let on_decision = move |decision: knot_acp::PermissionDecision| {
                    let Some(request) = &pending
                    else {
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
                let scroll_away_slot = Arc::clone(slot);
                let follow_slot = Arc::clone(slot);
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
                let panel_style = panel_view::PanelStyle { permission_risk,
                                                           markdown_font_size:
                                                               px(self.settings.markdown_font_size
                                                                  as f32),
                                                           mono_font_family: cx.theme()
                                                                               .mono_font_family
                                                                               .clone() };
                drop(state);
                drop(slot_guard);
                let scroll = self.panel_scroll_handle(id);
                let pending_context = self.panel_pending_context
                                          .get(&id)
                                          .cloned()
                                          .unwrap_or_default();
                let expanded = self.panel_input_expanded.contains(&id);
                // Per this same method's re-render-on-`take_dirty` poll
                // loop: each new streamed delta marks the session dirty
                // and triggers a repaint, so following the bottom here
                // (rather than via a dedicated scroll subscription) keeps
                // pace with streaming text.
                if should_follow {
                    scroll.scroll_to_bottom();
                }
                let input = self.panel_prompt_input(id, window, cx);
                // Offsets go negative scrolling down, so "not at the
                // bottom" is the remaining distance still being positive.
                // A conversation shorter than its viewport has no max
                // offset and so never shows the control.
                let scrolled_up =
                    scroll.max_offset().y + scroll.offset().y > px(SCROLL_BOTTOM_EPSILON);
                let scroll_to_bottom = scroll.clone();
                v_flex().size_full()
                        .child(div().relative()
                                    .flex_1()
                                    .min_h_0()
                                    .child(div().id("panel-conversation")
                                    .size_full()
                                    .overflow_y_scroll()
                                    .track_scroll(&scroll)
                                    .on_scroll_wheel(move |_: &gpui_kit::ScrollWheelEvent, _, _| {
                                        if let Ok(slot) = scroll_away_slot.lock()
                                           && let panel_session::PanelSessionSlot::Ready(handle) =
                                               &*slot
                                        {
                                            handle.clear_tracking();
                                        }
                                    })
                                    .child(panel_view::render_panel(&state_arc.lock().unwrap(),
                                                                    &scroll,
                                                                    &panel_style,
                                                                    on_decision,
                                                                    on_toggle_track)))
                                    .children(scrolled_up.then(|| {
                                        div().absolute()
                                             .bottom_3()
                                             .right_4()
                                             .child(Button::new("panel-scroll-to-bottom")
                                                 .icon(IconName::ChevronDown)
                                                 .tooltip("Scroll to latest")
                                                 .small()
                                                 .on_click(move |_: &ClickEvent, _, _| {
                                                     scroll_to_bottom.scroll_to_bottom();
                                                     // Jumping to the end also
                                                     // resumes following new
                                                     // output, which is what the
                                                     // control implies.
                                                     if let Ok(slot) = follow_slot.lock()
                                                        && let panel_session::PanelSessionSlot::Ready(handle) = &*slot
                                                     {
                                                         handle.set_tracking(true);
                                                     }
                                                 }))
                                    })))
                        .child(self.render_panel_input_area(id,
                                                            &input,
                                                            &pending_context,
                                                            expanded,
                                                            blocked,
                                                            turn_active,
                                                            &config_options,
                                                            cx))
                        .into_any_element()
            }
        }
    }

    /// The input area: attached-context chips, the expandable text entry,
    /// and a control row (add-context, permission mode, model, effort,
    /// expand/collapse, send) - a sibling of the message list under
    /// `render_panel_pane`, per design decision "Control bar placement".
    #[allow(clippy::too_many_arguments)]
    fn render_panel_input_area(&mut self, id: Uuid, input: &Entity<TextareaState>,
                               pending_context: &[PathBuf], expanded: bool, blocked: bool,
                               turn_active: bool, config_options: &[knot_acp::ConfigOption],
                               cx: &mut Context<Self>)
                               -> impl IntoElement {
        let can_send = !blocked && !turn_active && !input.read(cx).value().trim().is_empty();
        let shift_to_send = self.settings.agent_panel_shift_enter_sends;
        let send_tooltip = if shift_to_send {
            "Send (Shift+Enter)"
        }
        else {
            "Send (Enter)"
        };
        v_flex()
            .flex_shrink_0()
            .gap_2()
            .p_2()
            .border_t_1()
            .border_color(cx.theme().border)
            // Files and images dragged from Finder attach the same way the
            // paperclip and a pasted screenshot do, per `acp-panel-ui`'s
            // attached-context requirement.
            .drag_over::<gpui_kit::ExternalPaths>(|style, _, _, app| {
                style.bg(app.theme().accent)
            })
            .on_drop(cx.listener(move |view, paths: &gpui_kit::ExternalPaths, _, cx| {
                view.panel_pending_context
                    .entry(id)
                    .or_default()
                    .extend(paths.paths().iter().cloned());
                cx.notify();
            }))
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
                        div().flex_shrink_0()
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
                                    .on_click(cx.listener(
                                        move |view, _: &ClickEvent, _, cx| {
                                            view.toggle_panel_input_expanded(id, cx);
                                            cx.notify();
                                        },
                                    )),
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
    fn render_panel_config_selector(&self, id: Uuid, element_id: &'static str,
                                    placeholder: &'static str, disabled_tooltip: &'static str,
                                    option: Option<&knot_acp::ConfigOption>,
                                    cx: &mut Context<Self>)
                                    -> gpui_kit::AnyElement {
        let Some(option) = option
        else {
            return Button::new(element_id).label(placeholder)
                                          .tooltip(disabled_tooltip)
                                          .ghost()
                                          .small()
                                          .disabled(true)
                                          .into_any_element();
        };
        let current_value = option.current_value.as_str().unwrap_or_default();
        let current_label = option.options
                                  .iter()
                                  .find(|value| value.value == current_value)
                                  .map(|value| value.name.clone())
                                  .unwrap_or_else(|| option.name.clone());
        let config_id = option.id.clone();
        let values = option.options.clone();
        let entity = cx.entity();
        let session_arc = self.panel_sessions.get(&id).cloned();
        let is_permission_selector = element_id == PERMISSION_SELECTOR_ID;
        let selector_color = is_permission_selector.then(|| {
                                 panel_view::risk_color(panel_view::permission_risk_level(
                    current_value,
                    &option.name,
                ))
                             })
                             .flatten();
        let trigger = Button::new(element_id).label(current_label)
                                             .ghost()
                                             .small()
                                             .dropdown_caret(true)
                                             .when_some(selector_color, |button, color| {
                                                 button.text_color(rgb(color))
                                             });
        Popover::new(format!("{element_id}-{id}"))
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
            .content(move |_, window, app| {
                PopupMenu::build(window, app, |mut menu, _, _| {
                    for value in &values {
                        let entity = entity.clone();
                        let Some(session_arc) = session_arc.clone() else {
                            continue;
                        };
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
                        let mut item = item_color
                            .map(|color| {
                                let label = value.name.clone();
                                PopupMenuItem::element(move |_, _| {
                                    div().text_color(rgb(color)).child(label.clone())
                                })
                            })
                            .unwrap_or_else(|| PopupMenuItem::new(value.name.clone()));
                        if let Some(color) = item_color {
                            item = item.icon(
                                Icon::new(gpui_kit::assets::IconName::CircleDot)
                                    .text_color(rgb(color)),
                            );
                        }
                        menu = menu.item(item.on_click(move |_, _, app| {
                            let config_id = config_id.clone();
                            let value_id = value_id.clone();
                            let session_arc = Arc::clone(&session_arc);
                            entity.update(app, move |view, _cx| {
                                view.open_config_selector = None;
                                if let Ok(slot) = session_arc.lock()
                                    && let panel_session::PanelSessionSlot::Ready(handle) = &*slot
                                {
                                    let future = handle.set_config_option(config_id, value_id);
                                    let _guard = view.runtime.enter();
                                    view.runtime.spawn(future);
                                }
                            });
                        }));
                    }
                    menu
                })
            })
            .into_any_element()
    }

    /// Finds the declared config option matching one of `categories`
    /// (case-insensitive), for bucketing the agent's arbitrary option list
    /// into the input area's three fixed selector slots.
    pub(crate) fn find_config_option<'a>(options: &'a [knot_acp::ConfigOption],
                                         categories: &[&str])
                                         -> Option<&'a knot_acp::ConfigOption> {
        options.iter().find(|option| {
                          option.kind == "select"
                          && option.category.as_deref().is_some_and(|category| {
                                                           categories
                        .iter()
                        .any(|candidate| candidate.eq_ignore_ascii_case(category))
                                                       })
                      })
    }

    /// Gets or creates the conversation scroll handle for `id`'s panel.
    fn panel_scroll_handle(&mut self, id: Uuid) -> gpui_kit::ScrollHandle {
        self.panel_scroll_handles.entry(id).or_default().clone()
    }

    /// Opens the native file/image picker and attaches the chosen paths to
    /// `id`'s pending message, per `knot-ui-conventions`' native-picker
    /// rule (`cx.prompt_for_paths` over an in-app file browser).
    fn add_panel_context(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions { files:       true,
                                                             directories: false,
                                                             multiple:    true,
                                                             prompt:      Some("Attach".into()), });
        let this = cx.entity();
        cx.spawn(async move |_this, cx| {
              let Ok(Ok(Some(paths))) = receiver.await
              else {
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
        let Some(item) = cx.read_from_clipboard()
        else {
            return false;
        };
        let mut attached = false;
        for entry in item.entries {
            let ClipboardEntry::Image(image) = entry
            else {
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
        }
        else {
            self.panel_input_expanded.insert(id);
            true
        };
        // The cap is part of the textarea's own layout mode, so expanding
        // has to update the live entity rather than just the render height.
        let max_rows = if expanded {
            PANEL_INPUT_ROWS_EXPANDED
        }
        else {
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
        let Some(id) = self.selected_agent
        else {
            return;
        };
        if self.store
               .lock()
               .ok()
               .and_then(|store| store.agent(id).map(|agent| agent.view_mode))
           != Some(knot_core::ViewMode::Panel)
        {
            return;
        }
        let Some(slot) = self.panel_sessions.get(&id)
        else {
            return;
        };
        let Ok(slot) = slot.lock()
        else {
            return;
        };
        let panel_session::PanelSessionSlot::Ready(handle) = &*slot
        else {
            return;
        };
        let request = handle.state()
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
    fn panel_prompt_input(&mut self, id: Uuid, window: &mut Window, cx: &mut Context<Self>)
                          -> Entity<TextareaState> {
        if let Some(input) = self.panel_prompt_inputs.get(&id) {
            return input.clone();
        }
        let shift_to_send = self.settings.agent_panel_shift_enter_sends;
        let placeholder = Self::panel_prompt_placeholder();
        let max_rows = if self.panel_input_expanded.contains(&id) {
            PANEL_INPUT_ROWS_EXPANDED
        }
        else {
            PANEL_INPUT_ROWS_COLLAPSED
        };
        let input = cx.new(|cx| {
                          TextareaState::new(window, cx).placeholder(placeholder)
                                                        .submit_on_enter(!shift_to_send)
                                                        .auto_grow(1, max_rows)
                      });
        let subscription = cx.subscribe_in(&input,
                                           window,
                                           move |view: &mut Self, _, event, window, cx| {
                                               if let InputEvent::PressEnter { shift, .. } = event
                                                  && *shift == shift_to_send
                                               {
                                                   view.send_panel_prompt(id, window, cx);
                                               }
                                           });
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
        }
        else {
            "Enter to send, Shift+Enter for a newline"
        }
    }

    /// Reads and clears `id`'s prompt input, then sends it through the
    /// live ACP session (if any and not blocked on a pending permission),
    /// per `acp-panel-ui`'s "blocking further prompt submission until
    /// answered" requirement.
    fn send_panel_prompt(&mut self, id: Uuid, window: &mut Window, cx: &mut Context<Self>) {
        let Some(input) = self.panel_prompt_inputs.get(&id).cloned()
        else {
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
        let Some(slot) = self.panel_sessions.get(&id)
        else {
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
                             handle.session()
                         })
                }
                _ => None,
            }
        };
        let Some(session) = session
        else {
            self.panel_pending_context.insert(id, context);
            return;
        };
        cx.update_entity(&input, |state, cx| {
              state.set_value("", window, cx);
          });
        let _runtime_guard = self.runtime.enter();
        self.runtime.spawn(async move {
                        if let Err(error) = session.prompt(&text).await {
                            eprintln!("failed to send panel prompt: {error}");
                        }
                    });
        cx.notify();
    }

    /// Resizes `id`'s session grid/PTY to match the content pane's current
    /// size, if it changed.
    fn resize_session_to_pane(&mut self, id: Uuid, window: &Window, cx: &App) {
        let Some(session) = self.sessions.get(&id)
        else {
            return;
        };
        let (cell_width, cell_height) =
            terminal_cell_size(cx,
                               terminal_font_family(&self.settings, cx),
                               px(self.settings.terminal_font_size as f32));
        let viewport = window.viewport_size();
        let pane_width = (f32::from(viewport.width) - TERMINAL_SIDEBAR_WIDTH).max(cell_width);
        let pane_height = (f32::from(viewport.height) - TERMINAL_HEADER_HEIGHT).max(cell_height);
        let size = knot_terminal::GridSize { columns: (pane_width / cell_width) as usize,
                                             rows:    (pane_height / cell_height) as usize, };

        let current = session.lock()
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
        let Some(session) = self.sessions.get(&id)
        else {
            return;
        };
        let input = knot_terminal::KeyInput { key:      &keystroke.key,
                                              key_char: keystroke.key_char.as_deref(),
                                              control:  keystroke.modifiers.control,
                                              alt:      keystroke.modifiers.alt, };
        let Some(bytes) = knot_terminal::key_to_bytes(input)
        else {
            return;
        };
        let Ok(text) = String::from_utf8(bytes)
        else {
            return;
        };
        if let Ok(mut session) = session.lock() {
            let _ = session.send_text(&text);
        }
    }

    /// Converts a window-relative pixel position to a 0-indexed grid
    /// column/row, using the same pane geometry as `resize_session_to_pane`.
    fn grid_position(&self, position: gpui_kit::Point<gpui_kit::Pixels>, cx: &App)
                     -> (usize, usize) {
        let (cell_width, cell_height) =
            terminal_cell_size(cx,
                               terminal_font_family(&self.settings, cx),
                               px(self.settings.terminal_font_size as f32));
        let x = (f32::from(position.x) - TERMINAL_SIDEBAR_WIDTH).max(0.);
        let y = (f32::from(position.y) - TERMINAL_HEADER_HEIGHT).max(0.);
        ((x / cell_width) as usize, (y / cell_height) as usize)
    }

    /// Sends a mouse button press/release to the focused terminal pane's
    /// session, if the running program has enabled SGR mouse reporting -
    /// otherwise a no-op (falls back to no interaction rather than a
    /// scrollback/selection view, which isn't implemented yet).
    fn dispatch_mouse_button(&mut self, id: Uuid, position: gpui_kit::Point<gpui_kit::Pixels>,
                             button: knot_terminal::MouseButton, pressed: bool, cx: &App) {
        let Some(session) = self.sessions.get(&id)
        else {
            return;
        };
        let Some(grid) = session.lock().ok().and_then(|session| session.grid())
        else {
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
        let Some(bytes) = knot_terminal::mouse_to_bytes(knot_terminal::MouseInput { row,
                                                                                    column,
                                                                                    button,
                                                                                    pressed },
                                                        sgr)
        else {
            return;
        };
        if let (Ok(text), Ok(mut session)) = (String::from_utf8(bytes), session.lock()) {
            let _ = session.send_text(&text);
        }
    }

    /// Extends an in-progress text selection while the mouse is dragged
    /// with the left button held, when no mouse-aware program has claimed
    /// mouse reporting.
    fn dispatch_mouse_drag(&mut self, id: Uuid, position: gpui_kit::Point<gpui_kit::Pixels>,
                           cx: &App) {
        let Some(session) = self.sessions.get(&id)
        else {
            return;
        };
        let Some(grid) = session.lock().ok().and_then(|session| session.grid())
        else {
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
        let Some(session) = self.sessions.get(&id)
        else {
            return;
        };
        let Some(text) = session.lock()
                                .ok()
                                .and_then(|session| session.grid())
                                .and_then(|grid| grid.lock().unwrap().selection_text())
        else {
            return;
        };
        let text: String = text.lines()
                               .map(str::trim_end)
                               .collect::<Vec<_>>()
                               .join("\n");
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }

    /// Sends a scroll-wheel event to the focused terminal pane's session
    /// when the running program has enabled SGR mouse reporting.
    fn dispatch_scroll(&mut self, id: Uuid, position: gpui_kit::Point<gpui_kit::Pixels>,
                       lines: f32, cx: &App) {
        if lines == 0. {
            return;
        }
        let button = if lines > 0. {
            knot_terminal::MouseButton::WheelUp
        }
        else {
            knot_terminal::MouseButton::WheelDown
        };
        self.dispatch_mouse_button(id, position, button, true, cx);
    }

    fn create_agent(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let folder = self.new_agent_folder_input
                         .read(cx)
                         .value()
                         .trim()
                         .to_string();
        if folder.is_empty() || !PathBuf::from(&folder).is_dir() {
            self.error = Some("Choose an existing agent folder.".to_string());
            cx.notify();
            return false;
        }
        let name = self.new_agent_name_input
                       .read(cx)
                       .value()
                       .trim()
                       .to_string();
        let id = {
            let mut store = self.store.lock().unwrap();
            store.set_current_workspace(self.workspace_id);
            store.create(folder,
                         knot_agents::CreateOptions { name: (!name.is_empty()).then_some(name),
                                                      ..Default::default() })
        };
        if let Ok(store) = self.store.lock() {
            self.settings.saved_agents =
                store.saved_agents(self.settings.restore_conversation_on_launch);
            self.settings.saved_workspaces = store.saved_workspaces();
        }
        let _ = self.settings.persist();
        self.selected_agent = Some(id);
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
        open_agent_editor(Arc::clone(&self.store),
                          self.settings.clone(),
                          AgentEditorRequest { workspace_id:   self.workspace_id,
                                               prefill_folder: None,
                                               insert_after:   None,
                                               edit_target:    None, },
                          Self::select_and_focus_created_agent(cx),
                          cx);
    }

    /// An `on_created` callback for [`open_agent_editor`] that selects the
    /// new agent (and switches out of the dashboard, if it was open) so it
    /// becomes the visible agent in the sidebar and content pane, matching
    /// how tapping an existing agent already behaves.
    fn select_and_focus_created_agent(cx: &mut Context<Self>)
                                      -> impl Fn(Uuid, &mut Window, &mut App) + 'static {
        let weak = cx.entity().downgrade();
        move |id, _window, app| {
            if let Some(entity) = weak.upgrade() {
                entity.update(app, |view, cx| {
                          view.selected_agent = Some(id);
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
    avatar:       String,
    name:         String,
    folder:       String,
    header_title: String,
    agent_type:   String,
    state:        Option<(knot_agents::AgentState, Option<knot_git::DiffStats>)>,
}

impl WorkspaceWindow {
    /// The selected agent's diff stat, with only the figures colored -
    /// additions green, deletions red, the changed-file count blue - and
    /// the words around them left muted. The count's noun goes through
    /// `l10n::plural_noun` rather than a local `if count == 1`, so the
    /// word (and its form) comes from the locale catalog.
    fn render_diff_stats(stats: &knot_git::DiffStats, font_family: String,
                         font_size: gpui_kit::Pixels, cx: &Context<Self>)
                         -> gpui_kit::AnyElement {
        let muted = cx.theme().muted_foreground;
        let files = knot_core::l10n::plural_noun(stats.files_changed, "count.file", "count.files");
        h_flex().flex_shrink_0()
                .whitespace_nowrap()
                .font_family(font_family)
                .text_size(font_size)
                .text_color(muted)
                .gap_1()
                .items_baseline()
                .child(div().text_color(rgb(app_state::DIFF_ADDED_COLOR))
                            .child(format!("+{}", stats.insertions)))
                .child(div().text_color(rgb(app_state::DIFF_REMOVED_COLOR))
                            .child(format!("-{}", stats.deletions)))
                .child(h_flex().items_baseline()
                               .child(div().child("("))
                               .child(div().text_color(rgb(app_state::DIFF_FILES_COLOR))
                                           .child(stats.files_changed.to_string()))
                               .child(div().ml_1().child(format!("{files})"))))
                .into_any_element()
    }

    fn selected_agent_header(&self) -> Option<SelectedAgentHeader> {
        let id = self.selected_agent?;
        let store = self.store.lock().ok()?;
        let agent = store.agent(id)?;
        let stats = self.diff_stats
                        .lock()
                        .ok()
                        .and_then(|stats| stats.get(&id).copied())
                        .flatten();
        let state = (!agent.is_shell()).then_some((agent.state, stats));
        Some(SelectedAgentHeader { avatar: agent.avatar.clone(),
                                   name: agent.name.clone(),
                                   folder: shorten_path(&agent.folder),
                                   header_title: agent.header_title().to_string(),
                                   agent_type: agent.agent_type.clone(),
                                   state })
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
            let Some(workspace) = store.workspaces()
                                       .iter()
                                       .find(|workspace| workspace.id == self.workspace_id)
            else {
                return v_flex().size_full()
                               .child(TitleBar::new().border_color(gpui_kit::transparent_black()))
                               .child("Workspace no longer exists.");
            };
            let agents =
                workspace.agent_ids
                         .iter()
                         .filter_map(|id| store.agent(*id))
                         .map(|agent| {
                             let persona_name =
                                 agent.persona_id.and_then(|id| {
                                                     self.settings
                                                         .personas
                                                         .iter()
                                                         .find(|persona| persona.id == id)
                                                         .map(|persona| persona.name.clone())
                                                 });
                             AgentRow { id: agent.id,
                                        avatar: agent.avatar.clone(),
                                        name: agent.name.clone(),
                                        folder: agent.folder.clone(),
                                        state: agent.state,
                                        is_shell: agent.is_shell(),
                                        is_companion: agent.is_companion,
                                        header_title: agent.header_title().to_string(),
                                        persona_name,
                                        agent_type: agent.agent_type.clone() }
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
            let folder = self.store
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

        let agent_rows =
            agents.into_iter().map(
                                   |AgentRow { id,
                                               avatar,
                                               name,
                                               folder,
                                               state,
                                               is_shell,
                                               is_companion,
                                               header_title,
                                               persona_name,
                                               agent_type, }| {
                                       let menu_name = name.clone();
                                       let folder_name =
                                           PathBuf::from(&folder).file_name()
                                                                 .map(|name| {
                                                                     name.to_string_lossy()
                                                                         .into_owned()
                                                                 })
                                                                 .unwrap_or(folder);
                                       // Legacy/imported data may carry more
                                       // than one character;
                                       // clamp to a single grapheme so it can't
                                       // overflow the tile.
                                       let avatar = avatar.graphemes(true)
                                                          .next()
                                                          .unwrap_or("🤖")
                                                          .to_string();
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
                    .bg(if selected {
                        cx.theme().muted
                    } else {
                        cx.theme().transparent
                    })
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
                                    // The agent type reads as one of the
                                    // row's detail lines, directly under the
                                    // name and above the persona - not
                                    // right-aligned opposite it, where it
                                    // floated away from the name it
                                    // describes and crowded the state dot.
                                    .children((!is_shell).then(|| {
                                        h_flex()
                                            .w_full()
                                            .min_w_0()
                                            .gap_1()
                                            .items_center()
                                            .child(
                                                Icon::new(SettingsWindow::agent_type_icon(
                                                    &agent_type,
                                                ))
                                                .xsmall()
                                                .text_color(cx.theme().muted_foreground),
                                            )
                                            .child(
                                                div()
                                                    .font_family(ui_font_name.clone())
                                                    .text_size(ui_font_size)
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(SettingsWindow::agent_type_label(
                                                        &agent_type,
                                                    )),
                                            )
                                    }))
                                    .children(persona_name.map(|persona_name| {
                                        div()
                                            .font_family(ui_font_name.clone())
                                            .text_size(ui_font_size)
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(format!("👤 {persona_name}"))
                                    }))
                                    .child(
                                        div()
                                            .font_family(ui_font_name.clone())
                                            .text_size(ui_font_size)
                                            .text_color(cx.theme().muted_foreground)
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .text_ellipsis()
                                            .child(header_title),
                                    )
                                    .child(
                                        div()
                                            .font_family(ui_font_name.clone())
                                            .text_size(ui_font_size)
                                            .text_color(cx.theme().muted_foreground)
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .text_ellipsis()
                                            .child(folder_name),
                                    ),
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
                        view.selected_agent = Some(id);
                        view.ensure_session(id);
                        view.ensure_panel_session(id);
                        cx.notify();
                    }))
                    .context_menu({
                        let store = Arc::clone(&store_for_menu);
                        let settings = settings_for_menu.clone();
                        let window_entity = window_entity.clone();
                        let name = menu_name.clone();
                        move |menu, _window, _cx| {
                            let mut menu = menu.item(PopupMenuItem::new("Edit Agent…").on_click({
                                let store = Arc::clone(&store);
                                let settings = settings.clone();
                                let window_entity = window_entity.clone();
                                move |_, _window, app| {
                                    let window_entity = window_entity.clone();
                                    open_agent_editor(
                                        Arc::clone(&store),
                                        settings.clone(),
                                        AgentEditorRequest {
                                            workspace_id,
                                            prefill_folder: None,
                                            insert_after: None,
                                            edit_target: Some(id),
                                        },
                                        move |_id, _window, app| {
                                            window_entity.update(app, |_, cx| cx.notify());
                                        },
                                        app,
                                    );
                                }
                            }));

                            if !is_companion {
                                menu = menu.item(
                                    PopupMenuItem::new("New Shell Companion").on_click({
                                        let store = Arc::clone(&store);
                                        let window_entity = window_entity.clone();
                                        move |_, _window, app| {
                                            let created = store
                                                .lock()
                                                .unwrap()
                                                .create_shell_companion(id)
                                                .is_ok();
                                            if created {
                                                window_entity.update(app, |_, cx| cx.notify());
                                            }
                                        }
                                    }),
                                );

                                menu = menu.item(PopupMenuItem::new("Restart Agent").on_click({
                                    let store = Arc::clone(&store);
                                    let window_entity = window_entity.clone();
                                    let name = name.clone();
                                    move |_, window, app| {
                                        let store = Arc::clone(&store);
                                        let window_entity = window_entity.clone();
                                        let name = name.clone();
                                        // Deferred: a `PopupMenu` dismisses
                                        // itself right after running this
                                        // handler, and a dialog opened inline
                                        // goes down with it. Opening on the
                                        // next turn of the loop lets the menu
                                        // finish closing first.
                                        window.defer(app, move |window, app| {
                                        window.open_alert_dialog(app, move |alert, _, _| {
                                            let store = Arc::clone(&store);
                                            let window_entity = window_entity.clone();
                                            alert
                                                .title("Restart Agent")
                                                .description(format!(
                                                    "Restart \"{name}\"? Its session will be \
                                                     cleared."
                                                ))
                                                .confirm()
                                                .on_ok(move |_, _, app| {
                                                    let _ = store.lock().unwrap().restart(id);
                                                    window_entity.update(app, |view, cx| {
                                                        view.remove_session(id);
                                                        view.panel_states.remove(&id);
                                                        // `restart` clears the
                                                        // persisted session ids;
                                                        // write them out so a
                                                        // relaunch doesn't resume
                                                        // the session just dropped.
                                                        view.persist_agents();
                                                        cx.notify();
                                                    });
                                                    true
                                                })
                                        });
                                        });
                                    }
                                }));
                            }

                            menu.item(PopupMenuItem::new("Remove Agent").on_click({
                                let window_entity = window_entity.clone();
                                let name = name.clone();
                                move |_, window, app| {
                                    let window_entity = window_entity.clone();
                                    let name = name.clone();
                                    // See "Restart Agent" above: the dialog
                                    // has to outlive the menu's dismissal.
                                    window.defer(app, move |window, app| {
                                    window.open_alert_dialog(app, move |alert, _, _| {
                                        let window_entity = window_entity.clone();
                                        alert
                                            .title("Remove Agent")
                                            .description(format!(
                                                "Remove \"{name}\"? This closes its session and \
                                                 cannot be undone."
                                            ))
                                            .confirm()
                                            .on_ok(move |_, _, app| {
                                                window_entity.update(app, |view, cx| {
                                                    view.remove_agent(id);
                                                    cx.notify();
                                                });
                                                true
                                            })
                                    });
                                    });
                                }
                            }))
                        }
                    })
                                   },
            );

        let selected_header = self.selected_agent_header();

        let dashboard_workspace =
            is_dashboard.then(|| {
                            let store = self.store.lock().unwrap();
                            let workspace =
                                store.workspaces()
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
                                    (workspace.name.clone(),
                                     workspace.color_hex.clone(),
                                     dash_agents)
                                }
                                None => (String::new(), "#1B4FB2".to_string(), Vec::new()),
                            };
                            dashboard::DashboardWorkspace { id: self.workspace_id,
                                                            name,
                                                            color_hex,
                                                            agents: self.dashboard_sort
                                                                        .sorted(dash_agents) }
                        });

        let weak = cx.entity().downgrade();

        let dashboard_content =
            dashboard_workspace.map(|dashboard_workspace| {
                let on_agent_tap = {
                    let weak = weak.clone();
                    move |id: Uuid, _window: &mut Window, app: &mut gpui_kit::App| {
                        if let Some(entity) = weak.upgrade() {
                            entity.update(app, |view, cx| {
                                      view.selected_agent = Some(id);
                                      view.view_mode = WorkspaceViewMode::Terminal;
                                      view.ensure_session(id);
                                      view.ensure_panel_session(id);
                                      cx.notify();
                                  });
                        }
                    }
                };
                let on_workspace_nav =
                    |_id: Uuid, _window: &mut Window, _app: &mut gpui_kit::App| {};
                let on_add_agent = {
                    let weak = weak.clone();
                    let store = Arc::clone(&self.store);
                    move |workspace_id: Uuid, _window: &mut Window, app: &mut gpui_kit::App| {
                        let (folder, insert_after) =
                            store.lock()
                                 .ok()
                                 .and_then(|store| {
                                     store.workspaces()
                                          .iter()
                                          .find(|workspace| workspace.id == workspace_id)
                                          .map(|workspace| {
                                              let folder =
                                                  workspace.agent_ids
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
                                      let on_created =
                                          WorkspaceWindow::select_and_focus_created_agent(cx);
                                      open_agent_editor(Arc::clone(&view.store),
                                                        view.settings.clone(),
                                                        AgentEditorRequest { workspace_id,
                                                                             prefill_folder:
                                                                                 folder,
                                                                             insert_after,
                                                                             edit_target: None },
                                                        on_created,
                                                        cx);
                                  });
                        }
                    }
                };

                v_flex().size_full()
                        .child(div().size_full()
                                    .p_6()
                                    .overflow_hidden()
                                    .child(dashboard::workspace_section(dashboard_workspace,
                                                                        false,
                                                                        on_agent_tap,
                                                                        on_workspace_nav,
                                                                        on_add_agent)))
                        .into_any_element()
            });

        // Matches the Swift reference's title bar: it shows the selected
        // agent's identity directly (not a separate workspace-name strip
        // above a second header row) so the header abuts the traffic
        // lights with no redundant band, and a right-hand state/git-stats
        // indicator (`AgentFullHeader`) when a non-shell agent is selected.
        let title_bar_left = if is_dashboard {
            div().text_lg()
                 .child(knot_core::l10n::t("dashboard.title"))
                 .into_any_element()
        }
        else {
            match &selected_header {
                Some(header) => {
                    // The avatar and name always stay whole; the folder and
                    // the agent's status line give up space and ellipsize,
                    // the status line first since it is the longest and the
                    // least identifying.
                    h_flex().flex_1()
                            .min_w_0()
                            .items_center()
                            .gap_3()
                            .child(div().flex_shrink_0()
                                        .text_2xl()
                                        .child(header.avatar.clone()))
                            .child(div().flex_shrink_0()
                                        .text_lg()
                                        .font_semibold()
                                        .child(header.name.clone()))
                            .child(div().flex_shrink(1.)
                                        .min_w_0()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .text_ellipsis()
                                        .font_family(ui_font_name.clone())
                                        .text_size(ui_font_size)
                                        .text_color(cx.theme().muted_foreground)
                                        .child(header.folder.clone()))
                            .when(!header.header_title.is_empty(), |row| {
                                row.child(div().flex_shrink_0()
                                               .font_family(ui_font_name.clone())
                                               .text_size(ui_font_size)
                                               .text_color(cx.theme().muted_foreground)
                                               .child("●"))
                                   .child(div().flex_1()
                                               .min_w_0()
                                               .overflow_hidden()
                                               .whitespace_nowrap()
                                               .text_ellipsis()
                                               .font_family(ui_font_name.clone())
                                               .text_size(ui_font_size)
                                               .text_color(cx.theme().muted_foreground)
                                               .child(header.header_title.clone()))
                            })
                            .into_any_element()
                }
                None => div().text_lg()
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
            }).into_any_element()
        }
        else {
            match selected_header.as_ref()
                                 .and_then(|header| header.state.as_ref())
            {
                Some((state, git_stats)) => {
                    v_flex().items_end()
                            .gap_0p5()
                            .child(h_flex().items_center()
                                           .gap_2()
                                           .child(div().w(px(10.))
                                                       .h(px(10.))
                                                       .rounded_full()
                                                       .bg(state_color(*state)))
                                           .child(div().font_family(ui_font_name.clone())
                                                       .text_size(ui_font_size)
                                                       .text_color(cx.theme().muted_foreground)
                                                       .child(state_label(*state))))
                            .child(match git_stats {
                                       Some(stats) => Self::render_diff_stats(stats,
                                                                              ui_font_name.clone(),
                                                                              ui_font_size,
                                                                              cx),
                                       None => div().font_family(ui_font_name.clone())
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
                            .h(px(64.))
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
                            .child(v_flex().gap_1().p_4().children(agent_rows)),
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
                            .child(
                                SettingsWindow::icon_button(
                                    "workspace-dashboard",
                                    "icons/layout-dashboard.svg",
                                    "Dashboard",
                                    false,
                                )
                                .selected(is_dashboard)
                                .on_click(cx.listener(|view, _: &ClickEvent, _window, cx| {
                                    view.view_mode = match view.view_mode {
                                        WorkspaceViewMode::Terminal => {
                                            WorkspaceViewMode::Dashboard
                                        }
                                        WorkspaceViewMode::Dashboard => {
                                            WorkspaceViewMode::Terminal
                                        }
                                    };
                                    cx.notify();
                                })),
                            ),
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
                                                let is_panel_mode = {
                                                    let store = self.store.lock().unwrap();
                                                    store.agent(id).map(|agent| agent.view_mode)
                                                         == Some(knot_core::ViewMode::Panel)
                                                };
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
    }
}
