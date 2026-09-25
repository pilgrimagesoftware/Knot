//! The claim-refresh cache: a value the UI wants, computed somewhere that
//! must not run on the render path.
//!
//! GPUI re-renders on every keystroke, so anything a render computes runs at
//! that rate. A subprocess there is how the workspace window's header once
//! ran typing at the speed of `git`.
//!
//! The shape that answers it: a render asks the cache for what it already
//! has, and separately asks it to refresh what has aged out. Claiming hands
//! back a writer and marks the request immediately, so slow work cannot be
//! asked for twice; the work runs off the main thread and a later frame draws
//! the answer. The cache owns the values, the per-key request times and the
//! "something changed" flag. *How* the work gets off the main thread is the
//! caller's business, because the windows differ there - the workspace window
//! has a tokio runtime and a poll that redraws it, the command center has
//! neither and spawns through GPUI instead.
//!
//! Generic over the key and the value because there are two of these now:
//! `diff_stats` keys agents by id, `pull_request_state` keys pull requests by
//! URL. The polling and dirty-flag discipline is the same either way, and a
//! second hand-written copy is how one of them ends up subtly different.
//!
//! The maximum age is passed to [`RefreshCache::claim_refresh`] rather than
//! held here: it is a property of what is being refreshed, and the caller is
//! what knows its own cadence.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

/// Last known value per key, and when each was last requested.
pub(crate) struct RefreshCache<K, V> {
    values:    Arc<Mutex<BTreeMap<K, V>>>,
    /// Main-thread only: when each key's value was last *requested*, so the
    /// refresh runs on a cadence rather than once per render.
    requested: BTreeMap<K, Instant>,
    dirty:     Arc<AtomicBool>,
}

// Derived `Default` would demand `K: Default` and `V: Default`, which neither
// map needs - an empty cache is empty whatever it holds.
impl<K, V> Default for RefreshCache<K, V> {
    fn default() -> Self {
        Self { values:    Arc::new(Mutex::new(BTreeMap::new())),
               requested: BTreeMap::new(),
               dirty:     Arc::new(AtomicBool::new(false)), }
    }
}

impl<K: Ord + Clone, V: Clone + PartialEq> RefreshCache<K, V> {
    /// Claims a refresh of `key` if its value is older than `max_age`.
    ///
    /// Returns the writer the caller hands to whatever does the work, or
    /// `None` when the cached value is still fresh - which is the common
    /// case, since this is asked once per key per frame. Claiming marks the
    /// request immediately, so slow work cannot be asked for twice.
    pub(crate) fn claim_refresh(&mut self, key: K, max_age: Duration)
                                -> Option<RefreshWriter<K, V>> {
        let fresh = self.requested
                        .get(&key)
                        .is_some_and(|at| at.elapsed() < max_age);
        if fresh {
            return None;
        }
        self.requested.insert(key.clone(), Instant::now());
        Some(RefreshWriter { key,
                             values: Arc::clone(&self.values),
                             dirty: Arc::clone(&self.dirty) })
    }

    /// One key's cached value, or `None` when no answer has landed yet.
    ///
    /// A value type that is itself an `Option` therefore has two levels, and
    /// they mean different things: the outer one is whether an answer has
    /// landed at all, the inner one what that answer was. A caller that
    /// conflates them leaves a finished "there is nothing here" on
    /// "loading…" forever.
    pub(crate) fn get(&self, key: &K) -> Option<V> {
        self.values.lock().get(key).cloned()
    }

    /// Whether `key` has a cached value that `predicate` accepts.
    ///
    /// A read that does not clone, for a caller that asks something of every
    /// key each frame - whether an answer is final, say - and needs only a
    /// yes or no.
    pub(crate) fn holds(&self, key: &K, predicate: impl FnOnce(&V) -> bool) -> bool {
        self.values.lock().get(key).is_some_and(predicate)
    }

    /// Every cached value, copied out so a render can read them without
    /// holding the lock across the element tree it builds.
    pub(crate) fn snapshot(&self) -> BTreeMap<K, V> {
        self.values.lock().clone()
    }

    /// Whether a refresh has landed since this was last asked, clearing the
    /// flag. For the poll that decides whether to redraw.
    pub(crate) fn take_changed(&self) -> bool {
        self.dirty.swap(false, Ordering::SeqCst)
    }

    /// Drops everything remembered about `key`.
    ///
    /// Both maps are written from the render path, so without this every key
    /// the window has ever shown keeps an entry for the window's whole life -
    /// including ones that no longer exist.
    pub(crate) fn forget(&mut self, key: &K) {
        self.values.lock().remove(key);
        self.requested.remove(key);
    }

    /// Drops everything about every key the predicate rejects.
    pub(crate) fn retain(&mut self, keep: impl Fn(&K) -> bool) {
        self.values.lock().retain(|key, _| keep(key));
        self.requested.retain(|key, _| keep(key));
    }
}

/// The write half of one claimed refresh, sent to whatever thread does the
/// work. Carries no reference back to the window, so a refresh outliving its
/// window costs nothing.
pub(crate) struct RefreshWriter<K, V> {
    key:    K,
    values: Arc<Mutex<BTreeMap<K, V>>>,
    dirty:  Arc<AtomicBool>,
}

impl<K: Ord, V: PartialEq> RefreshWriter<K, V> {
    /// Records the result, flagging the cache as changed only when it
    /// actually differs - a value that came back the same must not cost a
    /// repaint, which is most refreshes.
    pub(crate) fn record(self, value: V) {
        let mut values = self.values.lock();
        let changed = values.get(&self.key) != Some(&value);
        values.insert(self.key, value);
        drop(values);
        if changed {
            self.dirty.store(true, Ordering::SeqCst);
        }
    }
}

#[cfg(test)]
mod tests;
