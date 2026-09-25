//! Unit tests for [`super`]. The row's text, which is the part that can be
//! wrong without anyone noticing on screen.

use knot_core::l10n;

use super::counts_label;
use crate::pull_request_state::PullRequestCounts;

fn counts(open: usize, merged: usize, closed: usize, pending: usize) -> PullRequestCounts {
    PullRequestCounts { open,
                        merged,
                        closed,
                        pending,
                        not_found: 0 }
}

/// Asserts the keys resolve and the values survive substitution, per
/// `.claude/rules/rust-structure.md` - never the English copy, which a copy
/// edit should be free to change.
#[test]
fn every_count_key_resolves_and_carries_its_number() {
    for key in ["pull_requests.counts_total",
                "pull_requests.count_open",
                "pull_requests.count_merged",
                "pull_requests.count_closed",
                "pull_requests.count_pending",
                "pull_requests.count_not_found"]
    {
        let text = l10n::t_with(key, &[("count", "7")]);
        assert_ne!(text, key, "{key} does not resolve");
        assert!(text.contains('7'), "{key} lost its count: {text}");
        assert!(!text.contains("%{"), "{key} left a placeholder: {text}");
    }
}

#[test]
fn a_full_breakdown_names_every_state_it_has() {
    let label = counts_label(counts(2, 1, 3, 4));

    for (key, count) in [("pull_requests.count_open", "2"),
                         ("pull_requests.count_merged", "1"),
                         ("pull_requests.count_closed", "3"),
                         ("pull_requests.count_pending", "4")]
    {
        assert!(label.contains(&l10n::t_with(key, &[("count", count)])),
                "{key} missing from {label}");
    }
}

/// A state nothing is in is a fact nobody needs and a third of the row's
/// width.
#[test]
fn a_state_with_no_pull_requests_is_left_out() {
    let label = counts_label(counts(2, 1, 0, 0));

    assert!(label.contains(&l10n::t_with("pull_requests.count_open", &[("count", "2")])));
    assert!(label.contains(&l10n::t_with("pull_requests.count_merged", &[("count", "1")])));
    assert!(!label.contains('0'),
            "a zero count reached the row: {label}");
}

#[test]
fn one_state_alone_gets_no_separator() {
    let label = counts_label(counts(3, 0, 0, 0));

    assert_eq!(label,
               l10n::t_with("pull_requests.count_open", &[("count", "3")]));
    assert!(!label.contains('·'), "{label}");
}

/// Showing "0 open · 0 merged · 0 closed" for records Knot has not asked
/// about states three things it does not know; the total states the one
/// thing it does.
#[test]
fn with_nothing_fetched_the_label_is_the_total() {
    let label = counts_label(counts(0, 0, 0, 4));

    assert_eq!(label,
               l10n::t_with("pull_requests.counts_total", &[("count", "4")]));
    assert!(!label.contains('0'), "no state may be claimed: {label}");
}

/// The order is fixed, so the row does not reshuffle as states change.
#[test]
fn the_states_keep_their_order() {
    let label = counts_label(counts(1, 1, 1, 1));
    let position = |key: &str| {
        label.find(&l10n::t_with(key, &[("count", "1")]))
             .unwrap_or_else(|| panic!("{key} missing from {label}"))
    };

    assert!(position("pull_requests.count_open") < position("pull_requests.count_merged"));
    assert!(position("pull_requests.count_merged") < position("pull_requests.count_closed"));
    assert!(position("pull_requests.count_closed") < position("pull_requests.count_pending"));
}

/// Counted apart, and left out when there are none - the same as every other
/// state, so a workspace with no missing pull requests never says so.
#[test]
fn not_found_is_shown_only_when_there_is_some() {
    let with = counts_label(PullRequestCounts { not_found: 2,
                                                ..counts(3, 0, 0, 0) });
    let without = counts_label(counts(3, 0, 0, 0));
    let not_found = l10n::t_with("pull_requests.count_not_found", &[("count", "2")]);

    assert!(with.contains(&not_found), "{with}");
    assert!(!with.contains(&l10n::t_with("pull_requests.count_pending", &[("count", "2")])),
            "not found counted as pending: {with}");
    assert!(!without.contains(&l10n::t_with("pull_requests.count_not_found", &[("count", "0")])),
            "{without}");
}
