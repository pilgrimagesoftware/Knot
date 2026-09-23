//! What the shared surface guarantees to a reader and to a writer.
//!
//! These construct with [`Settings::with_store_root`] rather than
//! [`Settings::default`]: nothing here persists, but the rule from #377 is
//! that a test near the store binds a root anyway, so a later edit that adds
//! a write cannot reach the user's real store.

use super::SharedSettings;
use crate::Settings;

/// The whole point of sharing: a write is visible to the next read.
#[test]
fn a_write_is_visible_to_a_later_read() {
    let shared = SharedSettings::new(Settings::with_store_root("/tmp/knot-shared-test"));

    shared.write(|settings| settings.mcp_server_port = 9000);

    assert_eq!(shared.read().mcp_server_port, 9000);
}

/// The other half, and the one that makes a frame coherent: a value already
/// in hand does not change underneath its holder.
#[test]
fn a_write_is_invisible_to_an_arc_taken_before_it() {
    let shared = SharedSettings::new(Settings::with_store_root("/tmp/knot-shared-test"));
    let before = shared.read();
    let port_before = before.mcp_server_port;

    shared.write(|settings| settings.mcp_server_port = 9000);

    assert_eq!(before.mcp_server_port, port_before,
               "a held Arc must not observe a later write");
    assert_eq!(shared.read().mcp_server_port, 9000, "but a fresh read must");
}

/// A writer starts from what the surface holds now, not from anything it
/// captured earlier - so it cannot revert a field it never touched. This is
/// the write half of #238 stated as a test.
#[test]
fn a_write_preserves_a_field_another_write_set() {
    let shared = SharedSettings::new(Settings::with_store_root("/tmp/knot-shared-test"));
    // A holder that read early and writes late, as an open window does.
    let stale = shared.read();

    shared.write(|settings| settings.mcp_server_port = 9000);
    shared.write(|settings| settings.ui_font_size = 18.0);

    let now = shared.read();
    assert_eq!(now.mcp_server_port, 9000,
               "the second write must not revert the first");
    assert_eq!(now.ui_font_size, 18.0);
    assert_ne!(stale.mcp_server_port, 9000,
               "the early read is still its own snapshot");
}

/// Handles are interchangeable: a clone is the same surface, not a copy of
/// it. Every window holding one has to see every other window's writes.
#[test]
fn a_cloned_handle_shares_the_same_surface() {
    let shared = SharedSettings::new(Settings::with_store_root("/tmp/knot-shared-test"));
    let other = shared.clone();

    other.write(|settings| settings.mcp_server_port = 9000);

    assert_eq!(shared.read().mcp_server_port, 9000);
}

/// The collections travel with the scalars. A write that touches a scalar
/// must not drop the roster, which is the failure `reload_preferences` had to
/// guard against by hand.
#[test]
fn a_scalar_write_keeps_the_collections() {
    let mut initial = Settings::with_store_root("/tmp/knot-shared-test");
    initial.recent_repos.push("/some/repo".to_string());
    let shared = SharedSettings::new(initial);

    shared.write(|settings| settings.mcp_server_port = 9000);

    let now = shared.read();
    assert_eq!(now.mcp_server_port, 9000);
    assert_eq!(now.recent_repos,
               vec!["/some/repo".to_string()],
               "a scalar write must leave the collections alone");
}

/// Two writers racing must not drop each other's change.
///
/// Each thread sets a field no other thread touches, so the only way a field
/// can read back wrong is a writer having installed a value it built from a
/// snapshot that another writer had already superseded. That makes the
/// assertion independent of scheduling: it holds for every interleaving, or
/// the compare-and-retry is missing. A plain `store` fails this.
#[test]
fn concurrent_writes_of_different_fields_all_survive() {
    use std::sync::{Arc, Barrier};

    const WRITERS: usize = 8;

    let shared = SharedSettings::new(Settings::with_store_root("/tmp/knot-shared-test"));
    // Start together, so the reads genuinely overlap rather than running in
    // sequence and passing for the wrong reason.
    let line = Arc::new(Barrier::new(WRITERS));

    std::thread::scope(|scope| {
        for writer in 0..WRITERS {
            let shared = shared.clone();
            let line = Arc::clone(&line);
            scope.spawn(move || {
                     line.wait();
                     shared.write(|settings| {
                               settings.recent_repos.push(format!("/repo/{writer}"));
                           });
                 });
        }
    });

    let now = shared.read();
    assert_eq!(now.recent_repos.len(),
               WRITERS,
               "every writer's push must survive; got {:?}",
               now.recent_repos);
    for writer in 0..WRITERS {
        assert!(now.recent_repos.contains(&format!("/repo/{writer}")),
                "writer {writer} was dropped: {:?}",
                now.recent_repos);
    }
}
