//! The one settings surface a running process reads, shared by every holder.
//!
//! Contract: `openspec/specs/settings-persistence/spec.md`.
//!
//! Every window used to own a [`Settings`] by value, taken when it opened.
//! That copy was a snapshot from that moment on, which is both halves of
//! #238: a preference changed elsewhere never reached it, and writing it back
//! reverted values it had never set. This holds one value instead, so there is
//! no copy to go stale.
//!
//! Sharing is copy-on-write rather than a mutex, for the render path.
//! `WorkspaceWindow` reads settings mid-render and GPUI re-renders per
//! keystroke, so a lock acquired per read would run at that rate - and with
//! reads nested inside renders that call further code that also reads
//! settings, a guard held across a sub-call is a deadlock rather than a
//! slowdown. [`SharedSettings::read`] is a refcount bump: it cannot block and
//! cannot deadlock. It also keeps this surface out of any lock-ordering
//! relationship with `AgentStore`'s mutex, which a dozen call sites already
//! hold while reading settings.
//!
//! The cost is that a write clones the whole value. That is a handful of
//! `Vec`s cloned on an explicit user action - saving a preference, importing,
//! editing the roster - and never on a frame.
//!
//! # The snapshot this does *not* reintroduce
//!
//! [`read`](SharedSettings::read) hands back an [`Arc<Settings>`] that does
//! not track later writes, which is what lets one frame draw a consistent set
//! of values. The snapshot that caused #238 was a `Settings` held for a
//! *window's lifetime*; this one is meant to be held for a frame or a
//! function. The distinction is lifetime, so the rule is: no struct field
//! holds one. Read it where it is needed.

use std::sync::Arc;

use arc_swap::ArcSwap;

use super::Settings;

/// A handle to the process's settings surface.
///
/// Cheap to clone - every clone reads and writes the same value.
#[derive(Debug, Clone)]
pub struct SharedSettings(Arc<ArcSwap<Settings>>);

impl SharedSettings {
    /// Take `settings` as the surface this handle shares.
    #[must_use]
    pub fn new(settings: Settings) -> Self {
        Self(Arc::new(ArcSwap::from_pointee(settings)))
    }

    /// The settings as they stand now.
    ///
    /// The returned value is fixed at the moment of the read: a write landing
    /// afterwards is invisible to it, which is what makes a frame that reads
    /// several preferences draw one consistent set rather than a mixture from
    /// either side of the write. Take a fresh one per frame; do not store it
    /// in a field.
    #[must_use]
    pub fn read(&self) -> Arc<Settings> {
        self.0.load_full()
    }

    /// Apply `change` to the current settings and install the result.
    ///
    /// The change is applied to a clone of whatever the surface holds *now*,
    /// never to a value the caller captured earlier. That is what makes the
    /// lost update unreachable: a writer cannot revert a field it did not
    /// touch, because it never had an old copy of it to write back.
    ///
    /// Returns the installed value, so a caller can persist the documents its
    /// change touched without reading the surface again.
    ///
    /// Installing is a compare-and-retry, not a plain store: if another
    /// writer lands between the read and the install, `change` runs again
    /// against the newer value rather than overwriting it. Without that, two
    /// writers racing would drop one of the two changes - the same lost
    /// update this type exists to remove, merely narrowed from a window's
    /// lifetime to a few instructions. `change` may therefore run more than
    /// once, which is why it is [`FnMut`]: keep it a pure edit of the value
    /// and put any side effect after the call. Uncontended - which, with GPUI
    /// writing from one thread, is every call today - it costs one extra
    /// comparison.
    pub fn write<F>(&self, mut change: F) -> Arc<Settings>
        where F: FnMut(&mut Settings) {
        self.0.rcu(|current| {
                  let mut next = Settings::clone(current);
                  change(&mut next);
                  next
              });
        self.read()
    }
}

#[cfg(test)]
mod tests;
