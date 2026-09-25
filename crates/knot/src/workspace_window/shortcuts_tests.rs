//! The window-scoped shortcuts, dispatched to a real workspace window
//! (`keybindings`: selecting an agent by number, focusing its input,
//! jumping to the latest output and toggling the panels), through the global
//! handlers the app registers.
//!
//! The agents use an agent type with no terminal process and no ACP
//! adapter, so selecting one - which activates it - starts nothing.
//!
//! Settings are rooted at a temporary directory; `open` persists the roster.

use std::sync::Arc;

use gpui_kit::Entity;
use gpui_kit::Focusable;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use crate::keymap::*;
use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceViewMode;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::pane_focus::FocusTarget;

/// Nothing launches for this: it is not a shell, and no adapter is
/// registered under it.
const INERT_AGENT_TYPE: &str = "keybindings-test-inert";

pub(super) struct Fixture {
    pub(super) window: VisualTestContext,
    pub(super) view:   Entity<WorkspaceWindow>,
    pub(super) agents: Vec<Uuid>,
    _dir:              TempDir,
}

pub(super) fn window_with_agents(count: usize, cx: &mut TestAppContext) -> Fixture {
    window_with(count, |_| {}, cx)
}

/// [`window_with_agents`], with `configure` applied to the settings first.
pub(super) fn window_with(count: usize, configure: impl FnOnce(&mut knot_core::Settings),
                          cx: &mut TestAppContext)
                          -> Fixture {
    let mut store = knot_agents::AgentStore::new();
    let space = crate::tests::workspace("Only");
    let workspace_id = space.id;
    store.add_workspace(space);
    for index in 0..count {
        store.create(format!("~/agent-{index}"),
                     knot_agents::CreateOptions { agent_type: Some(INERT_AGENT_TYPE.into()),
                                                  workspace_id: Some(workspace_id),
                                                  ..Default::default() });
    }
    // Sidebar order, which is what the digits count through - not creation
    // order, which `create` does not promise to keep.
    let agents = super::workspace_agent_ids(&store, workspace_id);
    assert_eq!(agents.len(), count);
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let dir = TempDir::new().expect("a temporary settings root");
    let view = cx.update(|cx| {
                     gpui_kit::init(cx);
                     WindowRegistry::install(cx);
                     // Bootstrap's, which the repaint poll reads once the
                     // clock is advanced past its first tick.
                     cx.set_global(crate::menu_bar::MenuBarState::default());
                     let mut settings = knot_core::Settings::with_store_root(dir.path());
                     configure(&mut settings);
                     crate::settings_global::install(settings, cx);
                     register_global_handlers(Arc::clone(&store), Arc::clone(&messages), cx);
                     WorkspaceWindow::open(store, messages, workspace_id, cx);
                     WindowRegistry::workspace_view(WindowKey::Workspace(workspace_id), cx)
                         .expect("opening a workspace registers its view")
                 });
    let handle = cx.update(|cx| *cx.windows().first().expect("the workspace window"));
    let window = VisualTestContext::from_window(handle, cx);
    Fixture { window,
              view,
              agents,
              _dir: dir }
}

impl Fixture {
    pub(super) fn press(&mut self, action: impl gpui_kit::Action) {
        self.window.dispatch_action(action);
        self.window.run_until_parked();
    }

    fn selected(&mut self) -> Option<Uuid> {
        self.view
            .read_with(&self.window, |view, _| view.selected_agent)
    }

    fn view_mode(&mut self) -> WorkspaceViewMode {
        self.view.read_with(&self.window, |view, _| view.view_mode)
    }

    pub(super) fn set_view_mode(&mut self, mode: WorkspaceViewMode) {
        self.view.update(&mut self.window, |view, cx| {
                     view.view_mode = mode;
                     cx.notify();
                 });
        self.window.run_until_parked();
    }
}

#[gpui_kit::test]
fn select_agent_n_selects_the_nth_row(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(3, cx);
    fixture.press(SelectAgent2);
    assert_eq!(fixture.selected(), Some(fixture.agents[1]));
    fixture.press(SelectAgent3);
    assert_eq!(fixture.selected(), Some(fixture.agents[2]));
}

#[gpui_kit::test]
fn select_agent_out_of_range_changes_nothing(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(2, cx);
    fixture.press(SelectAgent1);
    fixture.press(SelectAgent9);
    assert_eq!(fixture.selected(), Some(fixture.agents[0]));
}

#[gpui_kit::test]
fn select_agent_leaves_a_panel_for_the_agent_view(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(2, cx);
    fixture.set_view_mode(WorkspaceViewMode::Dashboard);
    fixture.press(SelectAgent1);
    assert_eq!(fixture.view_mode(), WorkspaceViewMode::Terminal);
    assert_eq!(fixture.selected(), Some(fixture.agents[0]));
}

#[gpui_kit::test]
fn the_panel_toggles_show_switch_and_hide(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    fixture.press(TogglePullRequests);
    assert_eq!(fixture.view_mode(), WorkspaceViewMode::PullRequests);
    fixture.press(ToggleDashboard);
    assert_eq!(fixture.view_mode(),
               WorkspaceViewMode::Dashboard,
               "switches panels");
    fixture.press(ToggleDashboard);
    assert_eq!(fixture.view_mode(),
               WorkspaceViewMode::Terminal,
               "a second press hides it");
}

/// Whether the agent's composer holds keyboard focus.
fn composer_focused(fixture: &mut Fixture, id: Uuid) -> bool {
    fixture.view
           .update_in(&mut fixture.window, |view, window, cx| {
               view.panel_prompt_input(id, window, cx)
                   .focus_handle(cx)
                   .is_focused(window)
           })
}

/// Moves focus to the window root without changing what is selected or
/// shown - standing in for a click into a search field or the git panel's
/// commit field, which leaves the frame's focus latch where it was.
fn focus_elsewhere(fixture: &mut Fixture) {
    fixture.view
           .update_in(&mut fixture.window, |view, window, cx| {
               window.focus(&view.root_focus.clone(), cx);
           });
    fixture.window.run_until_parked();
}

#[gpui_kit::test]
fn focus_agent_input_takes_focus_back_from_elsewhere_in_the_window(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let id = fixture.agents[0];
    fixture.press(SelectAgent1);
    assert_eq!(fixture.view
                      .read_with(&fixture.window, |view, _| view.focused_pane),
               Some(FocusTarget::Composer(id)),
               "the inert agent is Panel-mode, so its input is the composer");
    focus_elsewhere(&mut fixture);
    assert!(!composer_focused(&mut fixture, id),
            "the setup did not move focus away");

    fixture.press(FocusAgentInput);
    assert!(composer_focused(&mut fixture, id));
}

#[gpui_kit::test]
fn focus_agent_input_leaves_a_panel_for_the_composer(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let id = fixture.agents[0];
    fixture.press(SelectAgent1);
    fixture.set_view_mode(WorkspaceViewMode::Dashboard);
    focus_elsewhere(&mut fixture);

    fixture.press(FocusAgentInput);
    assert_eq!(fixture.view_mode(), WorkspaceViewMode::Terminal);
    assert!(composer_focused(&mut fixture, id));
}

#[gpui_kit::test]
fn focus_agent_input_does_nothing_without_a_selection(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(0, cx);
    fixture.set_view_mode(WorkspaceViewMode::Dashboard);
    fixture.press(FocusAgentInput);
    assert_eq!(fixture.view_mode(), WorkspaceViewMode::Dashboard);
}

/// A conversation list for the selected agent, scrolled to its top.
fn scrolled_up_list(fixture: &mut Fixture, id: Uuid) -> gpui_kit::ListState {
    let list = gpui_kit::ListState::new(20, gpui_kit::ListAlignment::Top, gpui_kit::px(0.));
    list.scroll_to(gpui_kit::ListOffset { item_ix:        0,
                                          offset_in_item: gpui_kit::px(0.), });
    let installed = list.clone();
    // Notified, the way the render that creates a real list is: the
    // shortcut's handler is registered from the frame that sees the list.
    fixture.view.update(&mut fixture.window, |view, cx| {
                    view.panel_lists.insert(id, installed);
                    cx.notify();
                });
    fixture.window.run_until_parked();
    list
}

#[gpui_kit::test]
fn jump_to_bottom_scrolls_the_conversation_to_its_end(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let id = fixture.agents[0];
    fixture.press(SelectAgent1);
    let list = scrolled_up_list(&mut fixture, id);

    fixture.press(JumpToBottom);
    assert_eq!(list.logical_scroll_top().item_ix, list.item_count());
}

#[gpui_kit::test]
fn jump_to_bottom_leaves_a_hidden_conversation_alone(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let id = fixture.agents[0];
    fixture.press(SelectAgent1);
    fixture.set_view_mode(WorkspaceViewMode::Dashboard);
    let list = scrolled_up_list(&mut fixture, id);

    fixture.press(JumpToBottom);
    assert_eq!(list.logical_scroll_top().item_ix, 0);
    assert_eq!(fixture.view_mode(), WorkspaceViewMode::Dashboard);
}

/// Whether macOS would draw `action`'s menu item enabled: it asks exactly
/// this of the focused window's last frame (`app-menu`).
fn available(fixture: &mut Fixture, action: &dyn gpui_kit::Action) -> bool {
    fixture.window
           .update(|window, cx| window.is_action_available(action, cx))
}

#[gpui_kit::test]
fn a_shortcut_that_would_do_nothing_is_unavailable(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(0, cx);
    assert!(available(&mut fixture, &ToggleDashboard));
    assert!(!available(&mut fixture, &SelectAgent1), "there is no agent");
    assert!(!available(&mut fixture, &FocusAgentInput),
            "nothing is selected");
    assert!(!available(&mut fixture, &JumpToBottom),
            "nothing is selected");
}

#[gpui_kit::test]
fn select_agent_is_available_up_to_the_agent_count(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(2, cx);
    assert!(available(&mut fixture, &SelectAgent2));
    assert!(!available(&mut fixture, &SelectAgent3),
            "there is no third agent");
    assert!(available(&mut fixture, &FocusAgentInput),
            "a window opens with an agent selected");
}

#[gpui_kit::test]
fn jump_to_bottom_is_unavailable_behind_a_panel(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let id = fixture.agents[0];
    fixture.press(SelectAgent1);
    scrolled_up_list(&mut fixture, id);
    assert!(available(&mut fixture, &JumpToBottom));

    fixture.set_view_mode(WorkspaceViewMode::Dashboard);
    assert!(!available(&mut fixture, &JumpToBottom));
}

/// The composer that held focus is not drawn behind a panel. Unless focus
/// moves to the window's root element, gpui resolves the stale handle to the
/// tree root - above every shortcut handler - and the menu goes dead.
#[gpui_kit::test]
fn the_shortcuts_stay_reachable_behind_a_panel(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(2, cx);
    let id = fixture.agents[0];
    fixture.press(SelectAgent1);
    assert!(composer_focused(&mut fixture, id));

    fixture.press(ToggleDashboard);
    assert!(available(&mut fixture, &SelectAgent2));
    assert!(available(&mut fixture, &ToggleDashboard));
}

/// ⌘T opens the agent editor - a window of its own - from the workspace
/// window, behind a panel too, and has nothing to act on elsewhere.
#[gpui_kit::test]
fn new_agent_opens_the_editor_even_behind_a_panel(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    assert!(available(&mut fixture, &crate::app_bootstrap::NewAgent));
    fixture.set_view_mode(WorkspaceViewMode::PullRequests);
    let before = cx.update(|cx| cx.windows().len());
    fixture.press(crate::app_bootstrap::NewAgent);
    assert_eq!(cx.update(|cx| cx.windows().len()), before + 1);
}
