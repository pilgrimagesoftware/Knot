//! Catalog coverage for the strings that would otherwise ship as their own
//! keys - a missing entry renders the key, which is visible but easy to
//! miss in review.

use super::*;

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
    assert_eq!(workspace_window::prompt_queue::queued_status_label(false),
               knot_core::l10n::t("panel.queued"));
    assert_eq!(workspace_window::prompt_queue::queued_status_label(true),
               knot_core::l10n::t("panel.failed"));
}
