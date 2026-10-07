//! How long ago something happened, and when, for the UI: the panel's
//! message timestamps (#577) and the sidebar status dot's last-message time
//! (#582) share these so the two read the same.

use std::time::SystemTime;

/// How long ago `sent_at` was, in words - the small label shown beside a
/// prompt or response, per issue #577. Buckets widen as the conversation
/// ages rather than ever showing a raw count of seconds, matching
/// `workspace_window::render::mcp_pane::taken_ago_text`'s shape.
pub(crate) fn relative_timestamp(sent_at: SystemTime) -> String {
    let elapsed = SystemTime::now().duration_since(sent_at)
                                   .unwrap_or_default();
    let total = elapsed.as_secs();

    if total < 60 {
        knot_core::l10n::t("panel.timestamp.just_now")
    }
    else if total < 3600 {
        knot_core::l10n::t_with("panel.timestamp.minutes_ago",
                                &[("minutes", &(total / 60).to_string())])
    }
    else if total < 86400 {
        knot_core::l10n::t_with("panel.timestamp.hours_ago",
                                &[("hours", &(total / 3600).to_string())])
    }
    else {
        knot_core::l10n::t_with("panel.timestamp.days_ago",
                                &[("days", &(total / 86400).to_string())])
    }
}

/// The absolute moment `sent_at` represents, for the timestamp's tooltip, in
/// the user's own time zone. Not localized: it is a fixed-format instant,
/// not a sentence. Falls back to UTC if the local offset cannot be read -
/// `current_local_offset` is unsound to call from more than one thread,
/// which a GPUI app always is, so a failure here is expected on some runs
/// rather than a bug to chase.
pub(crate) fn absolute_timestamp(sent_at: SystemTime) -> String {
    const FORMAT: &[time::format_description::FormatItem<'_>] =
        time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]");
    let offset = time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC);
    time::OffsetDateTime::from(sent_at).to_offset(offset)
                                       .format(FORMAT)
                                       .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    /// The bucket boundaries in `relative_timestamp` - just now, minutes,
    /// hours, days - per issue #577. Catalogue keys resolve regardless of
    /// locale, so the test checks which bucket was chosen, not the copy.
    #[test]
    fn relative_timestamp_buckets_by_elapsed_time() {
        let now = SystemTime::now();

        assert_eq!(relative_timestamp(now),
                   knot_core::l10n::t("panel.timestamp.just_now"));
        assert_eq!(relative_timestamp(now - Duration::from_secs(90)),
                   knot_core::l10n::t_with("panel.timestamp.minutes_ago", &[("minutes", "1")]));
        assert_eq!(relative_timestamp(now - Duration::from_secs(3 * 3600)),
                   knot_core::l10n::t_with("panel.timestamp.hours_ago", &[("hours", "3")]));
        assert_eq!(relative_timestamp(now - Duration::from_secs(2 * 86400)),
                   knot_core::l10n::t_with("panel.timestamp.days_ago", &[("days", "2")]));
    }
}
