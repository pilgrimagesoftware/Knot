//! Which store root a [`Settings`] writes to, and which constructors bind
//! one.
//!
//! Split from `store.rs` because it is the concern that decides whether a
//! value may write at all - separate from what it holds, how it is read, and
//! how each document is serialized.
//!
//! The rule the whole module exists to enforce: a `Settings` that was never
//! bound to a root cannot persist. See [`Settings::resolved_paths`].

use std::path::PathBuf;

use super::Settings;
use super::paths::StorePaths;
use crate::error::{Error, Result};

impl Settings {
    /// An empty settings value whose documents live under `dir`.
    pub fn with_store_root(dir: impl Into<PathBuf>) -> Self {
        Self { paths: Some(StorePaths::rooted(dir)),
               ..Self::default() }
    }

    /// Whether this value is bound to a store root, and so able to write.
    ///
    /// Exposed for the app's own diagnostics and for the tests that pin
    /// which constructors may persist - asserting on this rather than by
    /// attempting a write is what lets those tests run without going near
    /// the user's real store.
    #[must_use]
    pub fn is_bound(&self) -> bool {
        self.paths.is_some()
    }

    /// An empty settings value bound to the platform's own directories.
    ///
    /// The fallback for a failed [`Self::load`]: a read that did not work
    /// should not also leave the app unable to save. This is the only
    /// constructor that reaches the user's real store without having read
    /// it, which is why it is named rather than implied - see
    /// [`Self::resolved_paths`].
    #[must_use]
    pub fn platform_default() -> Self {
        Self { paths: StorePaths::platform(),
               ..Self::default() }
    }

    /// Where this value's documents live.
    ///
    /// `pub(super)` only because the writers that call it live in `store.rs`
    /// and `store/refresh.rs`; it is not part of the crate's surface.
    ///
    /// Deliberately does NOT fall back to the platform directories. It used
    /// to, which meant a `Settings::default()` that reached any `persist_*`
    /// wrote over the user's real workspaces, agents, personas, bench and
    /// recent repos - silently, because the fallback is exactly what a
    /// working save looks like. The rule "use `with_store_root` in a test
    /// that persists" was written in the module docs and still cost two
    /// stores, so the construction is now unable to do it rather than
    /// documented as not to.
    ///
    /// Everything that legitimately writes binds a root: [`Self::load`] and
    /// [`Self::load_from_root`] from the documents they read,
    /// [`Self::with_store_root`] from its argument, and
    /// [`Self::platform_default`] for the one case that wants the real
    /// directories without a successful load.
    pub(super) fn resolved_paths(&self) -> Result<StorePaths> {
        self.paths.clone().ok_or_else(|| {
                              Error::Config("settings are not bound to a store root, so there is \
                                             nothing to write to - construct with \
                                             `Settings::load`, `Settings::with_store_root` or \
                                             `Settings::platform_default`"
                                                                          .to_string())
                          })
    }
}
