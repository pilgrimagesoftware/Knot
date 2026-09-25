//! What the workspace-scoped configurable shortcuts do to a workspace
//! window: select an agent by number, focus its input, jump to the latest
//! output, toggle a panel
//! (`keybindings`), and the root-element handlers that dispatch them.
//! `keymap::handlers` says why these are not global.

use gpui_kit::Context;
use gpui_kit::InteractiveElement;

use crate::keymap::*;
use crate::workspace_window::WorkspaceViewMode;
use crate::workspace_window::WorkspaceWindow;

/// Which of the window's shortcuts would do something this frame, read once
/// per frame. A shortcut that would not gets no handler, which is what makes
/// macOS draw its View menu item disabled (`app-menu`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShortcutAvailability {
    /// How many Select agent N shortcuts have an agent to select.
    pub(crate) agents:         usize,
    pub(crate) focus_input:    bool,
    pub(crate) jump_to_bottom: bool,
}

impl WorkspaceWindow {
    pub(crate) fn shortcut_availability(&self) -> ShortcutAvailability {
        let store = self.store.lock();
        let agents = super::workspace_agent_ids(&store, self.workspace_id).into_iter()
                                                                          .filter(|id| {
                                                                              store.agent(*id)
                                                                                   .is_some()
                                                                          })
                                                                          .count();
        let selected = self.selected_agent.filter(|id| store.agent(*id).is_some());
        // The same test `jump_to_bottom` applies: a conversation list exists
        // only for an agent showing one, which a Terminal-mode agent is not.
        let has_conversation = selected.is_some_and(|id| self.panel_lists.contains_key(&id));
        ShortcutAvailability { agents,
                               focus_input: selected.is_some(),
                               jump_to_bottom: !self.view_mode.is_takeover() && has_conversation }
    }

    /// Registers the window's shortcut handlers on `el`, the root element,
    /// each only when [`ShortcutAvailability`] says it applies.
    pub(crate) fn with_shortcut_actions(el: gpui_kit::Div, available: ShortcutAvailability,
                                        cx: &mut Context<Self>)
                                        -> gpui_kit::Div {
        macro_rules! select_agent {
            ($el:expr, [$($action:ty => $index:expr),* $(,)?]) => {{
                let mut el = $el;
                $(
                    if $index < available.agents {
                        el = el.on_action(cx.listener(|view, _: &$action, _, cx| {
                                               view.select_agent_at($index, cx)
                                           }));
                    }
                )*
                el
            }};
        }
        let el = select_agent!(el,
                               [SelectAgent1 => 0,
                                SelectAgent2 => 1,
                                SelectAgent3 => 2,
                                SelectAgent4 => 3,
                                SelectAgent5 => 4,
                                SelectAgent6 => 5,
                                SelectAgent7 => 6,
                                SelectAgent8 => 7,
                                SelectAgent9 => 8]);
        let el = el.on_action(cx.listener(|view, _: &crate::app_bootstrap::NewAgent, _, cx| {
                                    view.open_new_agent_dialog(cx)
                                }))
                   .on_action(cx.listener(|view, _: &ToggleDashboard, _, cx| {
                                    view.toggle_view(WorkspaceViewMode::Dashboard, cx)
                                }))
                   .on_action(cx.listener(|view, _: &TogglePullRequests, _, cx| {
                                    view.toggle_view(WorkspaceViewMode::PullRequests, cx)
                                }));
        let el = if available.focus_input {
            el.on_action(cx.listener(|view, _: &FocusAgentInput, _, cx| view.focus_agent_input(cx)))
        }
        else {
            el
        };
        if available.jump_to_bottom {
            el.on_action(cx.listener(|view, _: &JumpToBottom, _, cx| view.jump_to_bottom(cx)))
        }
        else {
            el
        }
    }

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
