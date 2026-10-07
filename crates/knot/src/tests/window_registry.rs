//! One window per target, and a stale entry that clears itself
//! (`openspec/specs/window-lifecycle`).
//!
//! These assertions count windows and read the registry directly rather than
//! going through `WorkspaceWindow::open`, which starts sessions and a repaint
//! poll. What they cover is the registry's own contract: a repeat request
//! raises rather than opens, and a handle whose window has closed is noticed
//! and dropped instead of being handed out again.

use gpui_kit::AnyWindowHandle;
use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::Render;
use gpui_kit::TestAppContext;
use gpui_kit::Window;
use gpui_kit::WindowOptions;
use gpui_kit::component::Root;
use gpui_kit::div;
use uuid::Uuid;

use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::window_registry::activate_or_open;

/// A root view with no content of its own: these assertions count windows,
/// they do not read a rendered frame.
struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// Opens a window with no view of its own and reports its handle, standing in
/// for whatever a real caller would open.
pub(super) fn open_blank(cx: &mut gpui_kit::App) -> Option<AnyWindowHandle> {
    cx.open_window(WindowOptions::default(), |window, cx| {
          let view = cx.new(|_| Blank);
          cx.new(|cx| Root::new(view, window, cx))
      })
      .ok()
      .map(Into::into)
}

#[gpui_kit::test]
fn a_repeat_request_raises_rather_than_opening(cx: &mut TestAppContext) {
    let key = WindowKey::Workspace(Uuid::new_v4());
    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);

          assert!(activate_or_open(key, cx, open_blank),
                  "the first request should open a window");
          assert_eq!(cx.windows().len(), 1);

          for _ in 0..9 {
              assert!(!activate_or_open(key, cx, open_blank),
                      "a repeat request should raise the open window, not open another");
          }
          assert_eq!(cx.windows().len(),
                     1,
                     "ten requests for one workspace should leave one window");
      });
}

#[gpui_kit::test]
fn different_targets_get_different_windows(cx: &mut TestAppContext) {
    let first = WindowKey::Workspace(Uuid::new_v4());
    let second = WindowKey::Workspace(Uuid::new_v4());
    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);

          assert!(activate_or_open(first, cx, open_blank));
          assert!(activate_or_open(second, cx, open_blank));
          assert!(activate_or_open(WindowKey::CommandCenter, cx, open_blank));
          assert_eq!(cx.windows().len(), 3);
      });
}

#[gpui_kit::test]
fn a_closed_window_is_forgotten_and_opens_again(cx: &mut TestAppContext) {
    let key = WindowKey::CommandCenter;
    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);
          assert!(activate_or_open(key, cx, open_blank));
      });

    // Close it the way the user would, leaving the registry holding a handle
    // whose window is gone - the state no close observer is watching for.
    let window = cx.update(|cx| {
                       cx.windows()
                         .first()
                         .copied()
                         .expect("a window should be open")
                   });
    cx.update(|cx| {
          window.update(cx, |_, window, _| window.remove_window())
                .ok();
      });

    cx.update(|cx| {
          assert!(activate_or_open(key, cx, open_blank),
                  "a request after the window closed should open one, not raise the closed handle");
      });
}

#[gpui_kit::test]
fn a_stale_entry_is_dropped_on_the_miss(cx: &mut TestAppContext) {
    let key = WindowKey::WorkspaceManager;
    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);
          assert!(activate_or_open(key, cx, open_blank));
      });

    let window = cx.update(|cx| {
                       cx.windows()
                         .first()
                         .copied()
                         .expect("a window should be open")
                   });
    cx.update(|cx| {
          window.update(cx, |_, window, _| window.remove_window())
                .ok();
      });

    cx.update(|cx| {
          assert!(!WindowRegistry::activate(key, cx),
                  "activating a closed window should report that there was none");
          assert!(!WindowRegistry::activate(key, cx),
                  "the stale entry should have been dropped by the first miss");
      });
}

#[gpui_kit::test]
fn the_registry_is_total_without_being_installed(cx: &mut TestAppContext) {
    cx.update(|cx| {
          gpui_kit::init(cx);
          let key = WindowKey::CommandCenter;
          assert!(!WindowRegistry::activate(key, cx));
          assert!(WindowRegistry::workspace_view(key, cx).is_none());
          WindowRegistry::forget(key, cx);
      });
}

/// The Window menu's list: workspace windows that are open now, not every
/// entry the registry has not yet noticed is closed (#543).
#[gpui_kit::test]
fn open_workspaces_lists_live_workspace_windows_only(cx: &mut TestAppContext) {
    let (open, closed) = (Uuid::new_v4(), Uuid::new_v4());
    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);
          activate_or_open(WindowKey::Workspace(open), cx, open_blank);
          activate_or_open(WindowKey::Workspace(closed), cx, open_blank);
          activate_or_open(WindowKey::CommandCenter, cx, open_blank);
      });
    let closing = cx.update(|cx| cx.windows()[1]);
    cx.update(|cx| {
          closing.update(cx, |_, window, _| window.remove_window())
                 .expect("the window is open");
      });
    cx.run_until_parked();

    let listed = cx.update(|cx| WindowRegistry::open_workspaces(cx));

    assert_eq!(listed, vec![open]);
}
