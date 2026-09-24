//! What `reconcile_panel_send_chord` does, against a real window.
//!
//! Colocated rather than in `crate::tests` because the thing under test is
//! `pub(in crate::workspace_window)` and reaching it from outside would mean
//! widening the window's surface to suit a test.
//!
//! Three things are worth holding, and none of them is the happy path:
//!
//! - The comparison guard. `set_submit_on_enter` notifies unconditionally, so a
//!   reconcile that reported "changed" every time would notify every tick
//!   forever. That failure mode is a pegged CPU, not a failing assertion, so
//!   the return value is asserted rather than the notify.
//! - The write-before-read ordering: the field has to move even when there are
//!   no composers yet, or the first one built after a change is reconciled a
//!   second time for nothing.
//! - Every composer, not the selected one. An agent whose panel is not on
//!   screen still has a composer holding the flag, and it is exactly the one
//!   nobody would notice was wrong.
//!
//! Every `Settings` here is rooted at a temporary directory: `open`
//! persists the roster, and a `Settings::default()` carries no store paths,
//! so it would write over the developer's own workspaces and agents.

use std::sync::Arc;

use gpui_kit::TestAppContext;
use parking_lot::Mutex;
use tempfile::TempDir;
use uuid::Uuid;

use crate::settings_global;
use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::panel::composer::new_panel_input;
use crate::workspace_window::panel::composer::panel_input_max_rows;
use crate::workspace_window::panel::composer::sends_now;

/// A store holding one agentless workspace, and that workspace's id. No
/// agents, so no session or adapter subprocess starts.
fn store_with_one_workspace(
    )
    -> (Arc<Mutex<knot_agents::AgentStore>>, Arc<Mutex<knot_messaging::MessageStore>>, Uuid)
{
    let mut store = knot_agents::AgentStore::new();
    let workspace = knot_core::Workspace { id:        Uuid::new_v4(),
                                           name:      "Only".to_string(),
                                           color_hex: "#123456".to_string(),
                                           agent_ids: Vec::new(), };
    let id = workspace.id;
    store.add_workspace(workspace);
    (Arc::new(Mutex::new(store)), Arc::new(Mutex::new(knot_messaging::MessageStore::new())), id)
}

/// An open workspace window, with the setting starting at `shift_to_send`.
fn window(shift_to_send: bool, dir: &TempDir, cx: &mut gpui_kit::App)
          -> gpui_kit::Entity<WorkspaceWindow> {
    let (store, messages, id) = store_with_one_workspace();

    gpui_kit::init(cx);
    WindowRegistry::install(cx);
    let mut settings = knot_core::Settings::with_store_root(dir.path());
    settings.agent_panel_shift_enter_sends = shift_to_send;
    settings_global::install(settings, cx);

    WorkspaceWindow::open(Arc::clone(&store), Arc::clone(&messages), id, cx);
    WindowRegistry::workspace_view(WindowKey::Workspace(id), cx).expect("the window should open")
}

/// The hint under the prompt box and the predicate the keys go through have
/// to name the same chord as each other, on one setting.
///
/// Written by Knot 4, who raised it reviewing #431; moved here from
/// `crate::tests::panel_composer` because `panel_prompt_send_hint` is
/// `pub(in crate::workspace_window)` and this side of the wall can read it
/// without the helper being widened to suit a test.
///
/// They are two separate reads of `agent_panel_shift_enter_sends`:
/// `render_panel_input_area` hands the surface's value to
/// [`WorkspaceWindow::panel_prompt_send_hint`], and [`sends_now`] consults
/// the surface itself when the key arrives. #429 was exactly the state where
/// one of those had moved and the other had not - the hint said Shift+Enter
/// and Enter went on sending - so a test that changes the setting and checks
/// only one side cannot see the defect it is named for. This changes it and
/// checks both.
///
/// The English is never asserted, only that it moves. The copy belongs to
/// `panel_prompt_send_hint`, and pinning it here would fail on every
/// rewording without saying anything about the chord.
#[gpui_kit::test]
fn the_hint_and_the_send_chord_read_the_same_setting(cx: &mut TestAppContext) {
    let dir = TempDir::new().expect("a temporary settings root");

    cx.update(|cx| {
          gpui_kit::init(cx);
          let mut settings = knot_core::Settings::with_store_root(dir.path());
          settings.agent_panel_shift_enter_sends = false;
          settings_global::install(settings, cx);

          let mut hints = Vec::new();
          for shift_to_send in [false, true] {
              settings_global::write(cx, |settings| {
                  settings.agent_panel_shift_enter_sends = shift_to_send
              });

              // Read the way `render_panel_input_area` reads it. The hint
              // takes the value rather than fetching it, so the liveness
              // under test is the caller's, not the helper's.
              let live = settings_global::read(cx).agent_panel_shift_enter_sends;
              hints.push(WorkspaceWindow::panel_prompt_send_hint(live));

              assert!(sends_now(shift_to_send, cx),
                      "the chord the hint calls 'to send' has to send \
                       (agent_panel_shift_enter_sends={shift_to_send})");
              assert!(!sends_now(!shift_to_send, cx),
                      "and the chord it calls 'for a newline' has to not send \
                       (agent_panel_shift_enter_sends={shift_to_send})");
          }

          assert_ne!(hints[0], hints[1],
                     "the hint has to move with the setting; one that does not goes on naming a \
                      chord the predicate has stopped agreeing with, which is #429 over again \
                      with the two halves swapped");
      });
}

/// The guard, which is the whole reason the field exists. Its failure mode
/// is a repaint every tick for the life of the window, so it is asserted as
/// a return value rather than waited on.
#[gpui_kit::test]
fn an_unchanged_setting_reconciles_to_nothing(cx: &mut TestAppContext) {
    let dir = TempDir::new().expect("a temporary settings root");

    cx.update(|cx| {
          let view = window(false, &dir, cx);

          view.update(cx, |view, cx| {
                  assert!(!view.reconcile_panel_send_chord(cx),
                          "a window that has just opened is already in line with the setting it \
                           opened on");
                  assert!(!view.reconcile_panel_send_chord(cx),
                          "and it stays that way - a reconcile that reports a change every time \
                           notifies every tick, which is a pegged CPU rather than a failing \
                           test");
              });
      });
}

/// The field has to move even with no composers to set, or the first
/// composer built after a change is reconciled a second time for nothing.
#[gpui_kit::test]
fn a_changed_setting_reconciles_once_and_then_settles(cx: &mut TestAppContext) {
    let dir = TempDir::new().expect("a temporary settings root");

    cx.update(|cx| {
          let view = window(false, &dir, cx);

          settings_global::write(cx, |settings| settings.agent_panel_shift_enter_sends = true);

          view.update(cx, |view, cx| {
                  assert!(view.reconcile_panel_send_chord(cx),
                          "the setting moved, so the reconcile has something to report");
                  assert!(view.panel_input_send_chord,
                          "and the field has to record it whether or not there was a composer to \
                           set - otherwise the next tick reports the same change again");
                  assert!(!view.reconcile_panel_send_chord(cx),
                          "the change is spent; a second report would be the notify loop the \
                           guard exists to prevent");
              });
      });
}

/// Every composer the window has built, not the one whose panel is showing.
/// An agent in an unselected pane keeps its composer and its flag, and it is
/// the one nobody would notice was wrong.
#[gpui_kit::test]
fn the_reconcile_reaches_composers_for_agents_that_are_not_selected(cx: &mut TestAppContext) {
    let dir = TempDir::new().expect("a temporary settings root");

    cx.update(|cx| {
          let view = window(false, &dir, cx);
          let unselected = Uuid::new_v4();

          // Built the way `panel_prompt_input` builds one, and parked in the
          // map without ever being selected or drawn.
          view.update(cx, |view, cx| {
                  let handle = view.window_handle;
                  handle.update(cx, |_, window, cx| {
                            let input =
                                new_panel_input(false, panel_input_max_rows(false), window, cx);
                            view.panel_prompt_inputs.insert(unselected, input);
                        })
                        .expect("the window handle should still be live");
              });

          settings_global::write(cx, |settings| settings.agent_panel_shift_enter_sends = true);

          view.update(cx, |view, cx| {
                  assert!(view.reconcile_panel_send_chord(cx),
                          "a composer nobody selected is still a composer holding the old flag");
                  assert!(view.panel_prompt_inputs.contains_key(&unselected),
                          "and reconciling must set it rather than drop it - the buffer it holds \
                           is the user's unsent draft");
              });
      });
}
