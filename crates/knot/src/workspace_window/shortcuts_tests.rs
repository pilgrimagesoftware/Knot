//! The window-scoped shortcuts, dispatched to a real workspace window
//! (`keybindings`: selecting an agent by number, focusing its input, and
//! toggling the panels), through the global handlers the app registers.
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

struct Fixture {
    window: VisualTestContext,
    view:   Entity<WorkspaceWindow>,
    agents: Vec<Uuid>,
    _dir:   TempDir,
}

fn window_with_agents(count: usize, cx: &mut TestAppContext) -> Fixture {
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
                   crate::settings_global::install(knot_core::Settings::with_store_root(dir.path()),
                                                     cx);
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
    fn press(&mut self, action: impl gpui_kit::Action) {
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

    fn set_view_mode(&mut self, mode: WorkspaceViewMode) {
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
