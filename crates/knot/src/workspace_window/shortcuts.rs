//! What the workspace-scoped configurable shortcuts do to a workspace
//! window: select an agent by number, focus its input, jump to the latest
//! output, toggle a panel
//! (`keybindings`). Their handlers are in `keymap::handlers`, which says why
//! they are global.

use gpui_kit::Context;

use crate::workspace_window::WorkspaceViewMode;
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// Selects and shows the agent at `index` in sidebar order, the same as
    /// clicking its row; nothing when there are fewer agents.
    pub(crate) fn select_agent_at(&mut self, index: usize, cx: &mut Context<Self>) {
        let id = {
            let store = self.store.lock();
            super::workspace_agent_ids(&store, self.workspace_id).into_iter()
                                                                 .filter(|id| {
                                                                     store.agent(*id).is_some()
                                                                 })
                                                                 .nth(index)
        };
        if let Some(id) = id {
            self.reveal_agent(id, cx);
        }
    }

    /// Returns to the agent view and has the next frame focus the selected
    /// agent's composer or terminal.
    ///
    /// Clearing the focus latch is what makes this work when focus has moved
    /// within the same pane - to a search field, say: the frame's focus pass
    /// only acts on a change of target, and the target has not changed. It
    /// keeps that pass's guards, too: an open dialog or an expanded artifact
    /// panel still keeps focus where it is.
    pub(crate) fn focus_agent_input(&mut self, cx: &mut Context<Self>) {
        if self.selected_agent.is_none() {
            return;
        }
        self.view_mode = WorkspaceViewMode::Terminal;
        self.focused_pane = None;
        cx.notify();
    }

    /// Scrolls the selected agent's conversation to its latest output and
    /// resumes following it - what the panel's own "Scroll to latest"
    /// button does. Nothing for a Terminal-mode agent, whose pane always
    /// shows the bottom of its grid (the port has no scrollback view), or
    /// while a Dashboard or Pull Requests panel is showing, which has no
    /// conversation on screen to scroll.
    pub(crate) fn jump_to_bottom(&mut self, cx: &mut Context<Self>) {
        if self.view_mode.is_takeover() {
            return;
        }
        let Some(id) = self.selected_agent
        else {
            return;
        };
        let Some(list) = self.panel_lists.get(&id)
        else {
            return;
        };
        list.scroll_to_end();
        if let Some(slot) = self.panel_sessions.get(&id)
           && let crate::panel_session::PanelSessionSlot::Ready(handle) = &*slot.lock()
        {
            handle.set_tracking(true);
        }
        cx.notify();
    }

    /// Shows `target`, or returns to the agent view if it is showing, like
    /// the sidebar's Dashboard and Pull Requests rows.
    pub(crate) fn toggle_view(&mut self, target: WorkspaceViewMode, cx: &mut Context<Self>) {
        self.view_mode = self.view_mode.toggled(target);
        cx.notify();
    }
}
