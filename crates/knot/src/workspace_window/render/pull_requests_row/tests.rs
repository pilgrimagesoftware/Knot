//! Unit tests for [`super`]. The row's text, which is the part that can be
//! wrong without anyone noticing on screen.

use knot_core::l10n;

use super::counts_label;
use crate::pull_request_state::PullRequestCounts;

fn counts(open: usize, merged: usize, closed: usize, pending: usize) -> PullRequestCounts {
    PullRequestCounts { open,
                        merged,
                        closed,
                        pending }
}

/// Asserts the key resolves and the value survives substitution, per
/// `.claude/rules/rust-structure.md` - never the English copy, which a copy
/// edit should be free to change.
#[test]
fn the_breakdown_keys_resolve_and_carry_their_numbers() {
    assert_ne!(l10n::t("pull_requests.counts"), "pull_requests.counts");
    assert_ne!(l10n::t("pull_requests.counts_pending"),
               "pull_requests.counts_pending");
    assert_ne!(l10n::t("pull_requests.counts_total"),
               "pull_requests.counts_total");

    let label = counts_label(counts(2, 1, 3, 0));

    assert!(label.contains('2') && label.contains('1') && label.contains('3'),
            "{label}");
    assert!(!label.contains("%{"),
            "an unsubstituted placeholder: {label}");
}

/// Showing "0 open · 0 merged · 0 closed" for records Knot has not asked
/// about states three things it does not know; the total states the one
/// thing it does.
#[test]
fn with_nothing_fetched_the_label_is_the_total() {
    let label = counts_label(counts(0, 0, 0, 4));

    assert!(label.contains('4'), "{label}");
    assert!(!label.contains('0'), "no state may be claimed: {label}");
}

#[test]
fn a_partly_known_breakdown_reports_the_rest_as_pending() {
    let label = counts_label(counts(1, 1, 1, 1));

    assert_eq!(label,
               l10n::t_with("pull_requests.counts_pending",
                            &[("open", "1"),
                              ("merged", "1"),
                              ("closed", "1"),
                              ("pending", "1")]));
}

#[test]
fn a_fully_known_breakdown_says_nothing_about_pending() {
    let label = counts_label(counts(2, 1, 3, 0));

    assert_eq!(label,
               l10n::t_with("pull_requests.counts",
                            &[("open", "2"), ("merged", "1"), ("closed", "3")]));
}
