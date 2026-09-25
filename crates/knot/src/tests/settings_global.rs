//! The shared settings surface, as `knot` reaches it
//! (`openspec/specs/settings-persistence/spec.md`, "A preference change
//! applies to open windows").
//!
//! These pin the global itself - installed once, readable, writable through
//! `&App`, and the same surface for every holder. What each *window* does
//! with it is covered where that window is tested; this is the wiring
//! underneath them.
//!
//! Every `Settings` here is rooted at a temporary directory. Nothing below
//! persists, but the rule from #377 is that a test near the store binds a
//! root regardless, so a later edit cannot reach the developer's own store.

use gpui_kit::TestAppContext;
use tempfile::TempDir;

use crate::settings_global;

#[gpui_kit::test]
fn the_installed_surface_is_readable(cx: &mut TestAppContext) {
    let dir = TempDir::new().expect("a temporary settings root");
    let mut settings = knot_core::Settings::with_store_root(dir.path());
    settings.mcp_server_port = 9000;

    cx.update(|cx| {
          settings_global::install(settings, cx);

          assert_eq!(settings_global::read(cx).mcp_server_port, 9000);
      });
}

/// A write goes through `&App`, not `global_mut`: the surface is
/// interior-mutable, so a writer does not need exclusive access to the app.
#[gpui_kit::test]
fn a_write_through_the_global_is_visible_to_the_next_read(cx: &mut TestAppContext) {
    let dir = TempDir::new().expect("a temporary settings root");

    cx.update(|cx| {
          settings_global::install(knot_core::Settings::with_store_root(dir.path()), cx);

          settings_global::write(cx, |settings| settings.mcp_server_port = 9000);

          assert_eq!(settings_global::read(cx).mcp_server_port, 9000);
      });
}

/// Installing is idempotent, and the first caller wins. A second install
/// replacing the surface would hand whoever already holds the first a value
/// nothing else writes to - the per-window copy of #238, reintroduced by the
/// installer rather than by a window.
#[gpui_kit::test]
fn a_second_install_does_not_replace_the_surface(cx: &mut TestAppContext) {
    let dir = TempDir::new().expect("a temporary settings root");
    let mut first = knot_core::Settings::with_store_root(dir.path());
    first.mcp_server_port = 9000;
    let mut second = knot_core::Settings::with_store_root(dir.path());
    second.mcp_server_port = 7000;

    cx.update(|cx| {
          settings_global::install(first, cx);
          settings_global::install(second, cx);

          assert_eq!(settings_global::read(cx).mcp_server_port,
                     9000,
                     "the first installed surface must survive a second install");
      });
}

/// Two holders taken separately are one surface, which is the property every
/// window depends on: a preference written by one is read by all of them.
#[gpui_kit::test]
fn handles_taken_separately_share_one_surface(cx: &mut TestAppContext) {
    let dir = TempDir::new().expect("a temporary settings root");

    cx.update(|cx| {
          settings_global::install(knot_core::Settings::with_store_root(dir.path()), cx);
          let one = settings_global::handle(cx);
          let other = settings_global::handle(cx);

          one.write(|settings| settings.mcp_server_port = 9000);

          assert_eq!(other.read().mcp_server_port, 9000);
          assert_eq!(settings_global::read(cx).mcp_server_port, 9000);
      });
}
