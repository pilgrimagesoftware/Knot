//! The section's non-row copy: what a classification and a runtime read as.
//!
//! Every string here resolves through the catalogue. A process's own command
//! and identifier are data and are shown verbatim, which is why neither
//! appears in this file.

use std::time::Duration;

use knot_processes::Activity;

pub(super) fn activity_text(activity: Activity) -> String {
    knot_core::l10n::t(match activity {
                           Activity::Background => "processes.background",
                           Activity::Foreground => "processes.foreground",
                       })
}

/// Elapsed runtime, at the coarsest two units that say something.
///
/// One catalog entry per shape rather than units glued together here: where
/// the number goes relative to its unit is the translator's to decide.
pub(super) fn runtime_text(elapsed: Duration) -> String {
    let total = elapsed.as_secs();
    let (days, hours, minutes, seconds) =
        (total / 86_400, (total % 86_400) / 3600, (total % 3600) / 60, total % 60);

    if days > 0 {
        return knot_core::l10n::t_with("processes.runtime_days",
                                       &[("days", &days.to_string()),
                                         ("hours", &hours.to_string())]);
    }
    if hours > 0 {
        return knot_core::l10n::t_with("processes.runtime_hours",
                                       &[("hours", &hours.to_string()),
                                         ("minutes", &minutes.to_string())]);
    }
    if minutes > 0 {
        return knot_core::l10n::t_with("processes.runtime_minutes",
                                       &[("minutes", &minutes.to_string()),
                                         ("seconds", &seconds.to_string())]);
    }

    knot_core::l10n::t_with("processes.runtime_seconds",
                            &[("seconds", &seconds.to_string())])
}
