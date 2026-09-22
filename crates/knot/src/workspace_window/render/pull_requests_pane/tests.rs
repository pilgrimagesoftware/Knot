//! Unit tests for [`super`]: the text and the icon a row shows, which are
//! the parts that can be wrong without anyone noticing on screen.

use knot_forge::{CheckRollup, ForgeAvailability, PullRequestState, PullRequestStatus};

use super::{detail_line, forge_notice_text, status_icon};

fn state(status: PullRequestStatus, checks: Option<CheckRollup>) -> PullRequestState {
    PullRequestState { number: Some(42),
                       title: Some("Do the thing".to_string()),
                       status,
                       checks }
}

#[test]
fn a_full_row_names_its_number_state_and_checks() {
    let state = state(PullRequestStatus::Open, Some(CheckRollup::Passing));

    let line = detail_line(Some("#42"), Some(state.status), state.checks);

    assert!(line.contains("#42"), "{line}");
    assert!(line.contains(&knot_core::l10n::t("pull_requests.open")),
            "{line}");
    assert!(line.contains(&knot_core::l10n::t("pull_requests.checks_passing")),
            "{line}");
}

/// A `gh` too old to report a field, or a pull request with no checks: the
/// part that is absent is left out rather than shown as a blank.
#[test]
fn absent_parts_are_left_out_rather_than_blanked() {
    let line = detail_line(None, Some(PullRequestStatus::Merged), None);

    assert_eq!(line, knot_core::l10n::t("pull_requests.merged"));
}

/// A row whose state could not be fetched still lists; it says it is being
/// checked rather than claiming a state.
#[test]
fn a_row_with_no_state_says_so_rather_than_guessing() {
    let line = detail_line(None, None, None);

    assert_eq!(line, knot_core::l10n::t("pull_requests.pending"));
    assert_ne!(line, knot_core::l10n::t("pull_requests.open"));
}

/// Every key the row can show has to resolve, and none may leave a
/// placeholder behind.
#[test]
fn every_state_and_check_key_resolves() {
    for key in ["pull_requests.open",
                "pull_requests.draft",
                "pull_requests.merged",
                "pull_requests.closed",
                "pull_requests.pending",
                "pull_requests.checks_passing",
                "pull_requests.checks_failing",
                "pull_requests.checks_pending",
                "pull_requests.empty",
                "pull_requests.open_failed",
                "pull_requests.forge_missing",
                "pull_requests.forge_unauthenticated"]
    {
        let text = knot_core::l10n::t(key);
        assert_ne!(text, key, "{key} does not resolve");
        assert!(!text.contains("%{"), "{key} left a placeholder: {text}");
    }
}

#[test]
fn the_keys_with_values_substitute_them() {
    let opened_by = knot_core::l10n::t_with("pull_requests.opened_by", &[("name", "Scout")]);
    assert!(opened_by.contains("Scout"), "{opened_by}");
    assert!(!opened_by.contains("%{"), "{opened_by}");

    let failed = knot_core::l10n::t_with("pull_requests.forge_failed", &[("reason", "no route")]);
    assert!(failed.contains("no route"), "{failed}");
    assert!(!failed.contains("%{"), "{failed}");

    let body = knot_core::l10n::t_with("pull_requests.remove_body",
                                       &[("url", "https://github.com/a/b/pull/1")]);
    assert!(body.contains("https://github.com/a/b/pull/1"), "{body}");
    assert!(!body.contains("%{"), "{body}");
}

/// Draft gets its own icon: the point of a draft is that it is not ready,
/// which a plain open icon does not say.
#[test]
fn each_state_has_its_own_icon() {
    let icons = [status_icon(Some(PullRequestStatus::Draft)),
                 status_icon(Some(PullRequestStatus::Open)),
                 status_icon(Some(PullRequestStatus::Merged)),
                 status_icon(Some(PullRequestStatus::Closed)),
                 status_icon(None)];

    let mut unique = icons.to_vec();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(),
               icons.len(),
               "two states share an icon: {icons:?}");
}

// --- The availability message ----------------------------------------------

/// Task 6.4's three cases, driven by what a probe found. Each has a
/// different fix, so each has to say something different.
#[test]
fn each_unavailable_forge_states_its_own_reason() {
    let missing = forge_notice_text(&ForgeAvailability::Missing).expect("a message");
    let signed_out = forge_notice_text(&ForgeAvailability::Unauthenticated).expect("a message");
    let failed =
        forge_notice_text(&ForgeAvailability::Failed("no route to host".to_string())).expect("a message");

    assert_ne!(missing, signed_out);
    assert_ne!(signed_out, failed);
    assert_ne!(missing, failed);
    assert!(failed.contains("no route to host"), "{failed}");
    for text in [&missing, &signed_out, &failed] {
        assert!(!text.contains("%{"), "left a placeholder: {text}");
    }
}

/// A working `gh` says nothing: a banner on every healthy view is noise.
#[test]
fn a_ready_forge_says_nothing() {
    assert!(forge_notice_text(&ForgeAvailability::Ready).is_none());
}
