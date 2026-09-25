//! Unit tests for [`super`]: the text and the icon a row shows, which are
//! the parts that can be wrong without anyone noticing on screen.

use knot_forge::{
    CheckRollup, ForgeAvailability, Mergeability, PullRequestState, PullRequestStatus,
};

use super::{REMOVE_ICON, RowStatus, detail_line, forge_notice_text, state_color, status_icon};

fn state(status: PullRequestStatus, checks: Option<CheckRollup>) -> PullRequestState {
    PullRequestState { number: Some(42),
                       title: Some("Do the thing".to_string()),
                       status,
                       checks,
                       mergeable: Mergeability::Mergeable,
                       // No row renders it; these tests are about what a row
                       // renders.
                       merged_at: None }
}

#[test]
fn a_full_row_names_its_number_state_and_checks() {
    let state = state(PullRequestStatus::Open, Some(CheckRollup::Passing));

    let line = detail_line(Some("#42"), RowStatus::Known(state.status), state.checks);

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
    let line = detail_line(None, RowStatus::Known(PullRequestStatus::Merged), None);

    assert_eq!(line, knot_core::l10n::t("pull_requests.merged"));
}

/// A decided pull request's checks say nothing anyone will act on.
#[test]
fn a_merged_or_closed_row_leaves_its_checks_out() {
    for status in [PullRequestStatus::Merged, PullRequestStatus::Closed] {
        for checks in [CheckRollup::Passing,
                       CheckRollup::Failing,
                       CheckRollup::Pending]
        {
            let line = detail_line(Some("#42"), RowStatus::Known(status), Some(checks));

            assert_eq!(line.split(" · ").count(),
                       2,
                       "{status:?} {checks:?}: {line}");
        }
    }
}

/// A draft is still open, so its checks still matter.
#[test]
fn a_draft_row_keeps_its_checks() {
    let line = detail_line(None,
                           RowStatus::Known(PullRequestStatus::Draft),
                           Some(CheckRollup::Failing));

    assert!(line.contains(&knot_core::l10n::t("pull_requests.checks_failing")),
            "{line}");
}

/// A row whose state could not be fetched still lists; it says it is being
/// checked rather than claiming a state.
#[test]
fn a_row_with_no_state_says_so_rather_than_guessing() {
    let line = detail_line(None, RowStatus::Pending, None);

    assert_eq!(line, knot_core::l10n::t("pull_requests.pending"));
    assert_ne!(line, knot_core::l10n::t("pull_requests.open"));
}

/// The forge's answer is in, so the row says what it said rather than that
/// it is still checking.
#[test]
fn a_row_the_forge_cannot_find_says_not_found() {
    let line = detail_line(None, RowStatus::NotFound, None);

    assert_eq!(line, knot_core::l10n::t("pull_requests.not_found"));
    assert_ne!(line, knot_core::l10n::t("pull_requests.pending"));
}

/// A failed fetch is retried, so "checking" is still true of it; only an
/// answer of "does not exist" moves a row off pending.
#[test]
fn the_row_status_follows_the_lookup() {
    use crate::pull_request_state::PullRequestLookup;

    assert_eq!(RowStatus::of(None), RowStatus::Pending);
    assert_eq!(RowStatus::of(Some(&PullRequestLookup::Failed)),
               RowStatus::Pending);
    assert_eq!(RowStatus::of(Some(&PullRequestLookup::NotFound)),
               RowStatus::NotFound);
    assert_eq!(RowStatus::of(Some(&PullRequestLookup::Known(state(PullRequestStatus::Closed,
                                                                  None)))),
               RowStatus::Known(PullRequestStatus::Closed));
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
                "pull_requests.not_found",
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
    let icons = [status_icon(RowStatus::Known(PullRequestStatus::Draft)),
                 status_icon(RowStatus::Known(PullRequestStatus::Open)),
                 status_icon(RowStatus::Known(PullRequestStatus::Merged)),
                 status_icon(RowStatus::Known(PullRequestStatus::Closed)),
                 status_icon(RowStatus::NotFound),
                 status_icon(RowStatus::Pending)];

    let mut unique = icons.to_vec();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(),
               icons.len(),
               "two states share an icon: {icons:?}");
}

// --- The row's colour -------------------------------------------------------

/// Each reason an open pull request can or cannot land, and merged and
/// closed: states that must never collide, since the colour is the first
/// thing a row is read by. Conflicting and blocked share orange on purpose.
#[test]
fn each_state_wears_its_own_colour() {
    let all = [state_color(Some(PullRequestStatus::Open), Mergeability::Mergeable),
               state_color(Some(PullRequestStatus::Open), Mergeability::Conflicting),
               state_color(Some(PullRequestStatus::Open), Mergeability::Behind),
               state_color(Some(PullRequestStatus::Open), Mergeability::ChecksRunning),
               state_color(Some(PullRequestStatus::Merged), Mergeability::Unknown),
               state_color(Some(PullRequestStatus::Closed), Mergeability::Unknown)];

    for colour in all {
        assert!(colour.is_some(), "a fetched state earns a colour");
    }
    for (index, left) in all.iter().enumerate() {
        for right in &all[index + 1..] {
            assert_ne!(left, right, "two states share a colour");
        }
    }
}

/// The colours the view's feedback named: green mergeable, orange
/// conflicting, yellow behind, blue checks running.
#[test]
fn open_rows_wear_the_named_colours() {
    let colour = |mergeable| state_color(Some(PullRequestStatus::Open), mergeable);
    let rgb = |value| Some(gpui_kit::Hsla::from(gpui_kit::rgb(value)));

    assert_eq!(colour(Mergeability::Mergeable),
               rgb(crate::consts::COLOR_IDLE));
    assert_eq!(colour(Mergeability::Conflicting),
               rgb(crate::consts::COLOR_RUNNING));
    assert_eq!(colour(Mergeability::Behind),
               rgb(crate::consts::COLOR_PULL_REQUEST_BEHIND));
    assert_eq!(colour(Mergeability::ChecksRunning),
               rgb(crate::consts::COLOR_INPUT));
}

/// A row must not claim a colour it has not earned: no state fetched, or an
/// open pull request whose mergeability GitHub has not computed yet.
#[test]
fn an_unearned_colour_is_not_claimed() {
    assert_eq!(state_color(None, Mergeability::Unknown), None);
    assert_eq!(state_color(None, Mergeability::Mergeable), None);
    assert_eq!(state_color(Some(PullRequestStatus::Open), Mergeability::Unknown),
               None);
}

/// A draft cannot land, so it wears the blocked colour rather than a green
/// that would read as ready - the same orange as a conflict, since both need
/// work before they can land.
#[test]
fn a_draft_wears_the_blocked_colour() {
    assert_eq!(state_color(Some(PullRequestStatus::Draft), Mergeability::Blocked),
               state_color(Some(PullRequestStatus::Open), Mergeability::Blocked));
    assert_eq!(state_color(Some(PullRequestStatus::Open), Mergeability::Blocked),
               state_color(Some(PullRequestStatus::Open), Mergeability::Conflicting));
}

/// The tint is a state marker behind the row's text, not a fill competing
/// with it.
#[test]
fn the_background_tint_is_lighter_than_the_border() {
    const {
        assert!(crate::consts::PULL_REQUEST_ROW_TINT > 0.0);
        assert!(crate::consts::PULL_REQUEST_ROW_TINT < crate::consts::PULL_REQUEST_ROW_BORDER_TINT);
        assert!(crate::consts::PULL_REQUEST_ROW_BORDER_TINT <= 1.0);
    }
}

/// The far-right control has to read as a control rather than as another
/// status: every other icon in the row is one, and an X among them reads as
/// "failed". Beside a pull request it reads worse still - as "close this
/// pull request", the one thing Knot will never do.
#[test]
fn the_remove_control_does_not_wear_a_status_icon() {
    let statuses = [status_icon(RowStatus::Known(PullRequestStatus::Draft)),
                    status_icon(RowStatus::Known(PullRequestStatus::Open)),
                    status_icon(RowStatus::Known(PullRequestStatus::Merged)),
                    status_icon(RowStatus::Known(PullRequestStatus::Closed)),
                    status_icon(RowStatus::NotFound),
                    status_icon(RowStatus::Pending)];

    assert!(!statuses.contains(&REMOVE_ICON),
            "the remove control wears a status icon");
    assert!(REMOVE_ICON.contains("trash"),
            "{REMOVE_ICON} does not read as delete");
}

/// The icon says "delete"; the tooltip says delete from *what*, which is the
/// part that matters when the forge is one click away.
#[test]
fn the_remove_control_has_a_tooltip() {
    let text = knot_core::l10n::t("pull_requests.remove");

    assert_ne!(text, "pull_requests.remove");
    assert!(!text.is_empty());
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
