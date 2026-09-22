//! The per-agent `git diff --numstat` cache both dashboards read.
//!
//! `git diff` is a subprocess. GPUI re-renders on every keystroke, and a
//! dashboard draws one card per agent, so computing a stat where it is drawn
//! costs one subprocess per agent per frame - which is how the workspace
//! window's header once ran typing at the speed of `git`.
//!
//! So nothing computes a stat on the render path. A render asks this cache
//! for what it already has, and separately asks it to refresh what has aged
//! out; the refresh runs off the main thread and the answer is picked up by
//! a later frame.
//!
//! The mechanism is [`crate::refresh_cache`], which this was the first of
//! two users of - `pull_request_state` is the other. What lives here is the
//! naming: which key, which value, and the cadence in
//! [`consts::DIFF_STATS_MAX_AGE`].

use std::time::Duration;

use uuid::Uuid;

use crate::consts;
use crate::refresh_cache::RefreshCache;

/// Last known diff stat per agent, and when each was last requested.
///
/// The value's `Option` is the *lookup*: `None` means the folder is not a git
/// checkout, which is a finished answer and not a pending one. An agent
/// absent from the cache is the pending case.
pub(crate) type DiffStatsCache = RefreshCache<Uuid, Option<knot_git::DiffStats>>;

/// How stale a diff stat may be before a render asks for it again.
pub(crate) const MAX_AGE: Duration = consts::DIFF_STATS_MAX_AGE;

#[cfg(test)]
mod tests;
