//! Resolving an agent to the process Knot spawned for its session.
//!
//! Contract: `openspec/specs/agent-processes/spec.md` - "Every running agent
//! exposes a session root process".
//!
//! Two launch paths own a process each: `sessions` holds the PTY-backed
//! shells, `panel_sessions` the ACP adapters. An agent can hold both at once,
//! because a Panel agent switched to Terminal mid-turn keeps its ACP
//! connection alive beside the new PTY. Which one is the session root is
//! therefore decided by the agent's view mode, not by which map happens to
//! have an entry.

use knot_core::ViewMode;
use uuid::Uuid;

use super::window::WorkspaceWindow;

/// The agent's session root, given what each launch path reports for it.
///
/// `None` whenever the path that owns this agent has no live process: never
/// started, deactivated, mid-restart, or exited.
// UNWIRED(#337): called by `agent_session_root` below, which the processes
// section's sampling task is what finally calls.
#[allow(dead_code)]
pub(crate) fn session_root(view_mode: ViewMode, pty: Option<u32>, adapter: Option<u32>)
                           -> Option<u32> {
    match view_mode {
        ViewMode::Terminal => pty,
        ViewMode::Panel => adapter,
    }
}

// UNWIRED(#337): the whole impl is reached only from the processes section's
// sampling task, which lands with the section itself.
#[allow(dead_code)]
impl WorkspaceWindow {
    /// The process whose descendants this agent's processes section lists.
    pub(super) fn agent_session_root(&self, agent_id: Uuid) -> Option<u32> {
        let view_mode = self.store
                            .lock()
                            .agent(agent_id)
                            .map(|agent| agent.view_mode)?;

        session_root(view_mode,
                     self.terminal_root(agent_id),
                     self.adapter_root(agent_id))
    }

    fn terminal_root(&self, agent_id: Uuid) -> Option<u32> {
        self.sessions
            .get(&agent_id)
            .and_then(|session| session.lock().process_id())
    }

    fn adapter_root(&self, agent_id: Uuid) -> Option<u32> {
        self.panel_sessions
            .get(&agent_id)
            .and_then(|slot| slot.lock().process_id())
    }
}

#[cfg(test)]
mod tests {
    use knot_core::ViewMode;

    use super::session_root;

    #[test]
    fn a_shell_agent_roots_at_its_pty_child() {
        assert_eq!(session_root(ViewMode::Terminal, Some(4242), None),
                   Some(4242));
    }

    #[test]
    fn a_panel_agent_roots_at_its_adapter() {
        assert_eq!(session_root(ViewMode::Panel, None, Some(1717)), Some(1717));
    }

    #[test]
    fn an_agent_holding_both_roots_at_the_one_its_view_mode_names() {
        // A Panel agent switched to Terminal keeps its ACP connection; the
        // pane the user is looking at is the terminal, and so is the root.
        assert_eq!(session_root(ViewMode::Terminal, Some(4242), Some(1717)),
                   Some(4242));
        assert_eq!(session_root(ViewMode::Panel, Some(4242), Some(1717)),
                   Some(1717));
    }

    #[test]
    fn a_deactivated_agent_has_no_root() {
        assert_eq!(session_root(ViewMode::Terminal, None, None), None);
        assert_eq!(session_root(ViewMode::Panel, None, None), None);
    }

    #[test]
    fn an_agent_mid_restart_has_no_root() {
        // The old process is gone and the new one is not up: the owning path
        // reports nothing, and the other path's leftover is not a substitute.
        assert_eq!(session_root(ViewMode::Terminal, None, Some(1717)), None);
        assert_eq!(session_root(ViewMode::Panel, Some(4242), None), None);
    }
}
