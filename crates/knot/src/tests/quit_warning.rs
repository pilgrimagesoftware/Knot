//! Quitting while agents work. Contract:
//! `openspec/specs/quit-warning/spec.md`.
//!
//! The decision is asserted through `quit_guard::decide` rather than by
//! watching for termination: `TestPlatform::quit` is a no-op, so "the app
//! quit" leaves no trace a test can read. `decide` is the same branch
//! `request_quit` takes, one step before it acts on it.

use gpui_kit::component::{Root, WindowExt};
use gpui_kit::{
    AnyWindowHandle, AppContext, Context, IntoElement, Render, TestAppContext, Window,
    WindowOptions, div,
};
use knot_agents::{AgentState, AgentStore, CreateOptions};

use crate::quit_guard::{self, QuitDecision, QuitGuard, warning_message, working_agent_count};

/// A root view with no content: the assertions read `Root`'s dialog
/// stack, not the rendered frame.
struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// A store holding `working` agents in the Working state and `idle` agents
/// left as created.
fn store_with(working: usize, idle: usize) -> std::sync::Arc<parking_lot::Mutex<AgentStore>> {
    let mut store = AgentStore::new();
    for index in 0..working {
        let id = store.create(format!("/tmp/working-{index}"), CreateOptions::default());
        store.set_activated(id, true);
        store.set_state(id, AgentState::Running);
    }
    for index in 0..idle {
        store.create(format!("/tmp/idle-{index}"), CreateOptions::default());
    }
    std::sync::Arc::new(parking_lot::Mutex::new(store))
}

/// A test app with the guard installed over `store`, and one open window
/// for a dialog to land in.
fn app_with(store: std::sync::Arc<parking_lot::Mutex<AgentStore>>, cx: &mut TestAppContext)
            -> AnyWindowHandle {
    let window = cx.update(|cx| {
                       gpui_kit::init(cx);
                       cx.set_global(QuitGuard::new(store));
                       cx.open_window(WindowOptions::default(), |window, cx| {
                             let view = cx.new(|_| Blank);
                             cx.new(|cx| Root::new(view, window, cx))
                         })
                         .expect("failed to open the test window")
                   });
    window.into()
}

fn decide(cx: &mut TestAppContext) -> QuitDecision {
    cx.update(quit_guard::decide)
}

fn dialog_open(handle: AnyWindowHandle, cx: &mut TestAppContext) -> bool {
    handle.update(cx, |_, window, cx| window.has_active_dialog(cx))
          .expect("the test window went away")
}

// ---------------------------------------------------------------- counting

#[test]
fn only_activated_running_agents_count_as_working() {
    let mut store = AgentStore::new();
    // Every non-Working state, all activated: none of them is work that
    // quitting would interrupt.
    for state in [AgentState::Idle, AgentState::Input, AgentState::Error] {
        let id = store.create("/tmp/agent", CreateOptions::default());
        store.set_activated(id, true);
        store.set_state(id, state);
    }
    assert_eq!(working_agent_count(store.agents()),
               0,
               "only the Running state is work in progress");

    // Running but never activated: the state is stale, there is no
    // session behind it.
    let stopped = store.create("/tmp/stopped", CreateOptions::default());
    store.set_state(stopped, AgentState::Running);
    assert_eq!(working_agent_count(store.agents()),
               0,
               "an agent that was never activated has no session to lose");

    store.set_activated(stopped, true);
    assert_eq!(working_agent_count(store.agents()), 1);
}

// ------------------------------------------------------------- the decision

#[gpui_kit::test]
fn quitting_while_every_agent_is_idle_does_not_warn(cx: &mut TestAppContext) {
    let handle = app_with(store_with(0, 3), cx);

    assert_eq!(decide(cx), QuitDecision::Quit);

    cx.update(quit_guard::request_quit);
    cx.run_until_parked();
    assert!(!dialog_open(handle, cx),
            "an idle quit must not stop to ask");
}

#[gpui_kit::test]
fn quitting_with_one_working_agent_warns(cx: &mut TestAppContext) {
    let handle = app_with(store_with(1, 2), cx);

    assert_eq!(decide(cx), QuitDecision::Warn(1));

    cx.update(quit_guard::request_quit);
    cx.run_until_parked();
    assert!(dialog_open(handle, cx),
            "one working agent must hold the quit open");
}

#[gpui_kit::test]
fn quitting_with_several_working_agents_warns_about_all_of_them(cx: &mut TestAppContext) {
    let handle = app_with(store_with(3, 1), cx);

    assert_eq!(decide(cx), QuitDecision::Warn(3));

    cx.update(quit_guard::request_quit);
    cx.run_until_parked();
    assert!(dialog_open(handle, cx));
}

#[gpui_kit::test]
fn a_quit_request_with_no_guard_installed_is_not_blocked(cx: &mut TestAppContext) {
    // The global is installed during startup; a quit arriving before that
    // has nothing to protect and must not be trapped.
    cx.update(gpui_kit::init);
    assert_eq!(decide(cx), QuitDecision::Quit);
}

// -------------------------------------------------------------- the actions

#[gpui_kit::test]
fn cancelling_leaves_the_application_open_and_still_guarded(cx: &mut TestAppContext) {
    let handle = app_with(store_with(2, 0), cx);
    cx.update(quit_guard::request_quit);
    cx.run_until_parked();
    assert!(dialog_open(handle, cx));

    // Cancel is the dialog's own dismissal; what matters afterwards is
    // that nothing was consumed - the next quit meets the same guard.
    handle.update(cx, |_, window, cx| window.close_dialog(cx))
          .expect("the test window went away");
    cx.run_until_parked();

    assert!(!dialog_open(handle, cx), "cancel must close the warning");
    assert_eq!(decide(cx),
               QuitDecision::Warn(2),
               "cancelling must not arm the bypass");
}

#[gpui_kit::test]
fn confirming_quits_without_reopening_the_warning(cx: &mut TestAppContext) {
    let handle = app_with(store_with(2, 0), cx);
    cx.update(quit_guard::request_quit);
    cx.run_until_parked();
    assert!(dialog_open(handle, cx));

    cx.update(quit_guard::set_confirmed_for_test);
    assert_eq!(decide(cx),
               QuitDecision::Quit,
               "the confirmed quit must pass straight through");
}

#[gpui_kit::test]
fn the_confirmed_quit_bypass_is_spent_after_one_request(cx: &mut TestAppContext) {
    let _handle = app_with(store_with(1, 0), cx);

    cx.update(quit_guard::set_confirmed_for_test);
    assert_eq!(decide(cx), QuitDecision::Quit);
    assert_eq!(decide(cx),
               QuitDecision::Warn(1),
               "the bypass must cover exactly the request it was armed for");
}

#[gpui_kit::test]
fn agents_going_idle_while_the_warning_is_open_changes_neither_action(cx: &mut TestAppContext) {
    let store = store_with(2, 0);
    let handle = app_with(std::sync::Arc::clone(&store), cx);
    cx.update(quit_guard::request_quit);
    cx.run_until_parked();
    assert!(dialog_open(handle, cx),
            "the warning is open over 2 working agents");

    // The work finishes while the user is still looking at the dialog.
    {
        let mut store = store.lock();
        let ids = store.agents()
                       .iter()
                       .map(|agent| agent.id)
                       .collect::<Vec<_>>();
        for id in ids {
            store.set_state(id, AgentState::Idle);
        }
    }

    // The dialog is a snapshot of the request: it stays up, and the
    // user's choice still decides.
    assert!(dialog_open(handle, cx),
            "the warning must not vanish under the user");
    cx.update(quit_guard::set_confirmed_for_test);
    assert_eq!(decide(cx),
               QuitDecision::Quit,
               "confirming still quits once the agents have gone idle");
}

// ------------------------------------------------------------------- copy

#[test]
fn the_warning_copy_comes_from_the_catalog_and_names_the_count() {
    assert_eq!(quit_guard::warning_title(), "Agents are still working");
    assert_eq!(quit_guard::confirm_label(), "Quit Anyway");
    assert_eq!(quit_guard::cancel_label(), "Keep Working");

    // One agent is named, not counted: "1 agent" reads as a tally where
    // there is nothing to tally.
    let one = warning_message(1);
    assert!(one.starts_with("One agent is still working."), "{one}");
    assert!(!one.contains('1'),
            "the single-agent copy must not use a digit: {one}");

    for count in [2usize, 5, 17] {
        let many = warning_message(count);
        assert!(many.contains(&count.to_string()),
                "the warning must state how many agents are working: {many}");
        assert!(!many.contains("%{count}"),
                "the placeholder must be substituted: {many}");
    }
}
