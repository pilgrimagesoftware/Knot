//! Per-agent session lifecycle: starting a shell agent's PTY, tearing a
//! session (terminal or panel) down, and the off-thread diff-stat cache the
//! header and dashboard read.
//!
//! Teardown is the part worth keeping together. `WorkspaceWindow` holds a
//! dozen per-agent maps, and an agent leaving has to be removed from every
//! one of them; splitting that across files is how a session leak arrives.

use std::collections::BTreeMap;
use std::sync::Arc;

use knot_activity::EventSink;
use knot_git::Repository;
use knot_terminal::PtyTransport;
use knot_terminal::SessionConfig;
use knot_terminal::SessionPlan;
use knot_terminal::TerminalSession;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::app_state::apply_terminal_status;
use crate::panel_session;
use crate::panel_state;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::runs_a_terminal_process;

impl WorkspaceWindow {
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
    pub(super) fn ensure_session(&mut self, id: Uuid) {
        if self.sessions.contains_key(&id) {
            return;
        }
        let agent = {
            let store = self.store.lock();
            store.agent(id).cloned()
        };
        let Some(agent) = agent
        else {
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
                {
                    let mut last_output = on_output_activity.lock();
                    *last_output = Some(std::time::Instant::now());
                }
            },
            {
                let exited = Arc::clone(&self.exited_sessions);
                move |_status| {
                    {
                        let mut exited = exited.lock();
                        exited.push(id);
                    }
                }
            },
            move |event| match event {
                knot_terminal::GridEvent::Title(title) => {
                    {
                        let mut store = title_store.lock();
                        store.set_terminal_title(id, title);
                    }
                }
                knot_terminal::GridEvent::ClipboardStore(
                    knot_terminal::ClipboardType::Clipboard,
                    text,
                ) => {
                    {
                        let mut queue = clipboard_writes.lock();
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
                        let quiet = last_output.lock().is_some_and(|last_output| {
                                                          last_output.elapsed() >= QUIET_PERIOD
                                                      });
                        if quiet || start.elapsed() >= MAX_WAIT {
                            break;
                        }
                        std::thread::sleep(POLL_INTERVAL);
                    }
                    if let Err(error) = session.lock().start(&plan) {
                        eprintln!("failed to start terminal session: {error}");
                    }
                });
            }
            Err(error) => eprintln!("failed to start terminal session: {error}"),
        }
    }

    /// Requests a fresh diff stat for the selected agent if the cached one
    /// has aged out, computing it on the runtime's blocking pool.
    ///
    /// `git diff --numstat` is a subprocess, and this used to run inline in
    /// `selected_agent_header` - so every render spawned one, and since a
    /// keystroke in the prompt box re-renders, typing ran at the speed of
    /// `git`. `knot-git` is runtime-agnostic by contract, hence
    /// `spawn_blocking` rather than an async call.
    pub(super) fn refresh_diff_stats(&mut self, id: Uuid, folder: &str) {
        let Some(writer) = self.diff_stats.claim_refresh(id)
        else {
            return;
        };
        let folder = folder.to_string();
        let _runtime_guard = self.runtime.enter();
        self.runtime
            .spawn_blocking(move || writer.record(Repository::open(&folder).diff_stats().ok()));
    }

    /// Requests fresh diff stats for every agent the dashboard draws a card
    /// for, on the same TTL as the selected agent's.
    ///
    /// The dashboard shows one stat per agent, so computing them inline the
    /// way `selected_agent_header` once did costs a `git` subprocess *per
    /// card* per render rather than one - see [`Self::refresh_diff_stats`]
    /// for why that ran the UI at the speed of `git`. Companions get no
    /// card, so they are skipped here too.
    pub(super) fn refresh_dashboard_diff_stats(&mut self) {
        let folders = {
            let store = self.store.lock();
            store.workspaces()
                 .iter()
                 .find(|workspace| workspace.id == self.workspace_id)
                 .map(|workspace| {
                     workspace.agent_ids
                              .iter()
                              .filter_map(|id| store.agent(*id))
                              .filter(|agent| !agent.is_companion)
                              .map(|agent| (agent.id, agent.folder.clone()))
                              .collect::<Vec<_>>()
                 })
                 .unwrap_or_default()
        };
        for (id, folder) in folders {
            self.refresh_diff_stats(id, &folder);
        }
    }

    /// The cached diff stat per agent, copied out so a render can read it
    /// without holding the cache lock across the element tree it builds.
    pub(super) fn diff_stats_snapshot(&self) -> BTreeMap<Uuid, Option<knot_git::DiffStats>> {
        self.diff_stats.snapshot()
    }

    /// Tears down a session (e.g. its agent was removed or restarted).
    /// Covers both launch paths: the PTY terminal session for shell
    /// agents, and the ACP connection for Panel-mode ones - a non-shell
    /// agent has no `sessions` entry at all, so without the panel half
    /// removing it left its adapter subprocess running.
    pub(super) fn remove_session(&mut self, id: Uuid) {
        if let Some(session) = self.sessions.remove(&id) {
            let mut session = session.lock();
            // Best-effort: dropping the session is what kills the child, so
            // a failed shutdown leaks nothing. Logged because it means the
            // agent never saw the polite exit.
            if let Err(error) = session.shutdown() {
                eprintln!("failed to shut down agent {id}'s terminal: {error}");
            }
        }
        self.panel_phases.remove(&id);
        // Drop the list with the session: its scroll handler captured the
        // old slot, so a reconnect must build a fresh list bound to the new
        // one (and its row count must start empty so the reconciler
        // splices the conversation back in).
        self.panel_lists.remove(&id);
        self.panel_list_row_counts.remove(&id);
        if let Some(slot) = self.panel_sessions.remove(&id) {
            let handle = match std::mem::replace(&mut *slot.lock(),
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

    /// Drops a failed connection so the next render starts a fresh one.
    ///
    /// Only reachable from the `Failed` slot, which owns no handle and no
    /// subprocess - there is nothing to shut down, just the dead slot to
    /// clear so `ensure_panel_session` stops short-circuiting on it.
    pub(super) fn retry_panel_session(&mut self, id: Uuid) {
        self.panel_sessions.remove(&id);
        self.panel_phases.remove(&id);
        // The new connection gets a new slot; the old list's scroll handler
        // points at the dead one, so rebuild it.
        self.panel_lists.remove(&id);
        self.panel_list_row_counts.remove(&id);
        self.ensure_panel_session(id);
    }

    /// Tears `id`'s session and every piece of per-agent view state down,
    /// leaving the agent itself alone. Shared by [`Self::remove_agent`],
    /// which then drops the agent, and [`Self::deactivate_agent`], which
    /// does not - so a field added to one path cannot be missed in the
    /// other. That divergence is exactly the shape a session leak arrives in.
    pub(super) fn teardown_session(&mut self, id: Uuid) {
        self.remove_session(id);
        // Keyed by agent id and written from the render/poll path, so
        // without pruning here every agent the window has ever shown keeps
        // a diff-stat cache entry, a request timestamp and a nudge marker
        // for the window's whole life - including agents that no longer
        // exist. A stale nudge marker is not just memory: an id reused by a
        // recreated agent would inherit it and skip its first inbox prompt.
        self.diff_stats.forget(id);
        self.nudged_messages.remove(&id);
        self.forget_awaiting_notification(id);
        self.panel_states.remove(&id);
        self.panel_prompt_inputs.remove(&id);
        self.panel_prompt_input_subscriptions.remove(&id);
        self.panel_prompt_queues.remove(&id);
        self.panel_stopping.remove(&id);
        self.panel_lists.remove(&id);
        self.panel_list_row_counts.remove(&id);
        self.panel_pending_context.remove(&id);
        self.panel_input_expanded.remove(&id);
    }
}
