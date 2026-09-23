//! Which constructions of [`Settings`] may write, and which must refuse.
//!
//! The guard from #377. A `Settings::default()` used to reach the platform
//! directories through `resolved_paths`, so a test that persisted wrote over
//! the user's real store - silently, because a successful fallback is
//! indistinguishable from a successful save. These pin the boundary, so the
//! protection is a failing test rather than a remembered rule.

use crate::Settings;
use crate::error::Error;

/// The case that cost two stores: an unbound value must refuse rather than
/// find somewhere to write.
#[test]
fn an_unbound_settings_cannot_persist() {
    let settings = Settings::default();

    let err = settings.persist()
                      .expect_err("a default Settings must not reach the user's real store");

    assert!(matches!(err, Error::Config(_)),
            "expected a Config error naming the missing root, got {err:?}");
}

/// Every individual writer, not just `persist`, since a caller that knows
/// what it changed uses those directly and they each call `resolved_paths`.
#[test]
fn no_individual_writer_escapes_the_unbound_check() {
    let settings = Settings::default();

    assert!(settings.persist_preferences().is_err());
    assert!(settings.persist_roster().is_err());
    assert!(settings.persist_workspaces().is_err());
    assert!(settings.persist_personas().is_err());
    assert!(settings.persist_bench().is_err());
    assert!(settings.persist_recent_repos().is_err());
}

/// The other half: a bound value still writes, so the guard above is not
/// passing merely because persistence broke.
#[test]
fn a_rooted_settings_still_persists() {
    let dir = tempfile::tempdir().expect("a temporary store root");
    let settings = Settings::with_store_root(dir.path());

    settings.persist().expect("a rooted Settings persists");

    assert!(std::fs::read_dir(dir.path()).into_iter()
                                         .flatten()
                                         .next()
                                         .is_some(),
            "persisting should have written at least one document");
}

/// `platform_default` is the one constructor that reaches the real
/// directories without a successful load, and the app's fallback depends on
/// it being bound. Asserted through the public surface rather than by
/// writing, so the test cannot touch the user's store.
#[test]
fn platform_default_is_bound_to_a_root() {
    // `None` only when no home directory resolves, which is not the case
    // anywhere this suite runs.
    assert!(Settings::platform_default().is_bound(),
            "the app's fallback after a failed load must still be able to save");
    assert!(!Settings::default().is_bound());
}
