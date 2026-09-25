//! The git panel's commit window is themed like every other window.
//!
//! `Root` is what applies the theme's text colour and UI font to a window's
//! contents. The commit window once returned its view unwrapped, and drew its
//! title and message black in the dark theme, with GPUI's default font on its
//! buttons.

use gpui_kit::TestAppContext;
use gpui_kit::component::Root;
use tempfile::TempDir;

use crate::commit_window::{CommitWindow, open_commit_window};

#[gpui_kit::test]
fn the_commit_window_is_wrapped_in_root(cx: &mut TestAppContext) {
    let dir = TempDir::new().expect("a temporary settings root");
    cx.update(|cx| {
          gpui_kit::init(cx);
          crate::settings_global::install(knot_core::Settings::with_store_root(dir.path()), cx);
          open_commit_window(|_, _, _, _| {}, cx);
      });
    cx.run_until_parked();

    cx.update(|cx| {
          let window = cx.windows()
                         .into_iter()
                         .next()
                         .expect("the commit window opened");
          let root = window.downcast::<Root>()
                           .expect("the commit window's root is a Root");
          let holds_commit_view = root.read(cx)
                                      .expect("the commit window is open")
                                      .view()
                                      .clone()
                                      .downcast::<CommitWindow>()
                                      .is_ok();
          assert!(holds_commit_view, "the Root holds the commit view");
      });
}
