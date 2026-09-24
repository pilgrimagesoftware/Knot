//! Why an expanded section has no rows to show.
//!
//! Contract: `openspec/specs/agent-processes/spec.md` - "The section states
//! why it is empty".

use knot_processes::DescendantProcess;

/// Why an expanded section has no rows to show.
///
/// The spec forbids rendering blank: each case reads differently to the user,
/// and "nothing here" without saying which is the one that looks broken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EmptyState {
    /// No session root: never started, deactivated, mid-restart.
    NotRunning,
    /// Running, and its tree is empty.
    NothingSpawned,
    /// No sample has completed yet.
    Counting,
}

impl EmptyState {
    pub(super) fn text(self) -> String {
        match self {
            Self::NotRunning => knot_core::l10n::t("processes.empty_not_running"),
            Self::NothingSpawned => knot_core::l10n::t("processes.empty_none"),
            Self::Counting => knot_core::l10n::t("processes.count_unknown"),
        }
    }
}

/// Which empty state an expanded section is in, or `None` when it has rows.
pub(super) fn empty_state(is_running: bool, processes: Option<&[DescendantProcess]>)
                          -> Option<EmptyState> {
    if !is_running {
        return Some(EmptyState::NotRunning);
    }

    match processes {
        None => Some(EmptyState::Counting),
        Some([]) => Some(EmptyState::NothingSpawned),
        Some(_) => None,
    }
}
