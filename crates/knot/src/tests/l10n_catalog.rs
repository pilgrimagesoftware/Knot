//! Catalog coverage for the strings that would otherwise ship as their own
//! keys - a missing entry renders the key, which is visible but easy to
//! miss in review.

use crate::workspace_window;

/// Every word the About window shows comes from the catalog, so a missing
/// key would ship the key string itself where the version, copyright or a
/// credit line belongs.
#[test]
fn about_window_labels_resolve() {
    for key in ["about.title",
                "about.version_label",
                "about.build_label",
                "about.commit_unknown",
                "about.copy_details",
                "about.close",
                "about.copyright",
                "about.derived_from",
                "about.credits.author",
                "about.credits.license",
                "about.credits.built_with",
                "about.credits.toolkit",
                "about.credits.terminal",
                "about.credits.fonts"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

/// The queued-row controls carry no visible text of their own, so their
/// tooltips and accessibility labels are the only thing naming them - a
/// missing key would ship the key string itself as the button's name.
#[test]
fn queued_message_control_labels_resolve() {
    for key in ["panel.queued",
                "panel.queued_inbox_nudge",
                "panel.failed",
                "panel.retry",
                "panel.retry_queued",
                "panel.delete_queued",
                "panel.edit_queued",
                "panel.replace_composer_title",
                "panel.replace_composer_body",
                "panel.retry_connect"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

#[test]
fn the_queued_status_label_follows_the_failed_mark() {
    use workspace_window::prompt_queue::{PromptOrigin, queued_status_label};

    assert_eq!(queued_status_label(false, PromptOrigin::User),
               knot_core::l10n::t("panel.queued"));
    assert_eq!(queued_status_label(true, PromptOrigin::User),
               knot_core::l10n::t("panel.failed"));
}

/// A queued nudge is labelled as one, so a prompt the user never typed does
/// not read as one they did - except when it failed, where the state the
/// user must act on wins.
#[test]
fn a_queued_inbox_nudge_says_where_it_came_from() {
    use workspace_window::prompt_queue::{PromptOrigin, queued_status_label};

    assert_eq!(queued_status_label(false, PromptOrigin::InboxNudge),
               knot_core::l10n::t("panel.queued_inbox_nudge"));
    assert_ne!(queued_status_label(false, PromptOrigin::InboxNudge),
               queued_status_label(false, PromptOrigin::User));
    assert_eq!(queued_status_label(true, PromptOrigin::InboxNudge),
               knot_core::l10n::t("panel.failed"));
}
