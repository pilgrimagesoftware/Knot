//! The File menu's "Import…" item opens the Import window, once
//! (`openspec/specs/import-ui`).
//!
//! macOS routes a menu item through `App::dispatch_action`, which runs the
//! handler *inside* `active_window.update(...)`.
//! `TestAppContext::dispatch_action` dispatches through that same window
//! update, which is what makes these tests cover the menu path rather than a
//! straight call to the handler.
//!
//! The handler under test is registered by
//! `import_window::register_import_action`, the same call `run` makes, so the
//! window handle these assertions exercise is the one the app installs.

use gpui_kit::component::Root;
use gpui_kit::{
    AnyWindowHandle, AppContext, Context, IntoElement, Render, TestAppContext, Window,
    WindowOptions, div,
};
use uuid::Uuid;

use crate::app_bootstrap::OpenImport;
use crate::import_window::ImportWindow;
use crate::import_window::register_import_action;

/// A root view with no content of its own: these assertions count windows,
/// they do not read a rendered frame.
struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// The active window the menu dispatches through, plus the registered
/// handler.
fn app_with_one_window(cx: &mut TestAppContext) -> AnyWindowHandle {
    cx.update(|cx| {
          gpui_kit::init(cx);
          register_import_action(cx);
          let window = cx.open_window(WindowOptions::default(), |window, cx| {
                             let view = cx.new(|_| Blank);
                             cx.new(|cx| Root::new(view, window, cx))
                         })
                         .expect("failed to open the test window");
          window.into()
      })
}

fn window_count(cx: &mut TestAppContext) -> usize {
    cx.update(|cx| cx.windows().len())
}

#[gpui_kit::test]
fn the_import_window_opens_when_the_menu_dispatches_into_the_active_window(cx: &mut TestAppContext)
{
    let handle = app_with_one_window(cx);
    let before = window_count(cx);

    cx.dispatch_action(handle, OpenImport);
    cx.run_until_parked();

    assert_eq!(window_count(cx),
               before + 1,
               "Import… dispatched from the menu opened no window");
}

#[gpui_kit::test]
fn choosing_import_twice_does_not_open_a_second_window(cx: &mut TestAppContext) {
    let handle = app_with_one_window(cx);

    cx.dispatch_action(handle, OpenImport);
    cx.run_until_parked();
    let after_first = window_count(cx);

    cx.dispatch_action(handle, OpenImport);
    cx.run_until_parked();

    assert_eq!(window_count(cx),
               after_first,
               "choosing Import… again opened a second window instead of raising the open one");
}

#[gpui_kit::test]
fn import_opens_again_after_its_window_is_closed(cx: &mut TestAppContext) {
    let handle = app_with_one_window(cx);
    let before = window_count(cx);

    cx.dispatch_action(handle, OpenImport);
    cx.run_until_parked();

    // The Import window is the one that was not there before.
    let import = cx.update(|cx| {
                       *cx.windows()
                          .iter()
                          .find(|window| **window != handle)
                          .expect("the Import window is not among the app's windows")
                   });
    import.update(cx, |_, window, _| window.remove_window())
          .expect("the Import window went away before it could be closed");
    cx.run_until_parked();
    assert_eq!(window_count(cx),
               before,
               "closing the Import window left it open");

    cx.dispatch_action(handle, OpenImport);
    cx.run_until_parked();

    assert_eq!(window_count(cx),
               before + 1,
               "Import… did not reopen after its window was closed");
}

/// A workspace row says what comes with it, so the choice is made on what it
/// actually brings across rather than on a name alone.
#[test]
fn a_workspace_import_label_names_its_agent_count() {
    let mut workspace = knot_core::Workspace { id:                    Uuid::new_v4(),
                                               name:                  "WIP".into(),
                                               color_hex:             "#46A857".into(),
                                               agent_ids:             vec![Uuid::new_v4(),
                                                                           Uuid::new_v4()],
                                               layout_mode:           "single".into(),
                                               active_agent_ids:      Vec::new(),
                                               focused_pane_index:    0,
                                               split_ratio:           0.5,
                                               split_ratio_secondary: None,
                                               show_dashboard:        None,
                                               is_detached:           None,
                                               window_bounds:         None, };

    assert_eq!(ImportWindow::workspace_label(&workspace), "WIP (2 agents)");

    workspace.agent_ids.pop();
    assert_eq!(ImportWindow::workspace_label(&workspace), "WIP (1 agent)");

    workspace.agent_ids.clear();
    assert_eq!(ImportWindow::workspace_label(&workspace), "WIP (0 agents)");
}
