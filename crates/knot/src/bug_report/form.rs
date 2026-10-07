//! When Report is available, and what the dialog says about the last
//! attempt. Pure, so the rules are tested without a window.

use super::screenshots::Handoff;

/// Where the dialog stands. Everything but [`Phase::Submitting`] leaves the
/// report editable: a failure or a browser hand-off keeps what was typed, so
/// the user can retry or copy it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Phase {
    Editing,
    /// An issue is being filed. Report stays disabled until it finishes, so
    /// one report cannot become two issues.
    Submitting,
    /// Filing through `gh` failed, with why.
    Failed(String),
    /// The forge was not ready, and the compose page opened in the browser,
    /// with the report's screenshots handed over as this says.
    BrowserReady(Handoff),
    /// The forge was not ready, and the browser could not be opened either.
    BrowserFailed,
}

impl Phase {
    /// The status line under the form, if there is one to show.
    pub(super) fn message(&self) -> Option<String> {
        match self {
            Self::Editing => None,
            Self::Submitting => Some(knot_core::l10n::t("bug_report.submitting")),
            Self::Failed(error) => {
                Some(knot_core::l10n::t_with("bug_report.failed", &[("error", error)]))
            }
            Self::BrowserReady(handoff) => {
                Some(knot_core::l10n::t(match handoff {
                                            Handoff::None => "bug_report.browser_ready",
                                            Handoff::Shown => {
                                                "bug_report.browser_ready_screenshots"
                                            }
                                            Handoff::ByHand => {
                                                "bug_report.browser_ready_screenshots_by_hand"
                                            }
                                        }))
            }
            Self::BrowserFailed => Some(knot_core::l10n::t("bug_report.browser_failed")),
        }
    }

    /// Whether the message reports something that went wrong.
    pub(super) fn is_error(&self) -> bool {
        matches!(self, Self::Failed(_) | Self::BrowserFailed)
    }
}

/// The confirmation for an issue filed at `url`, saying what to do with its
/// screenshots when it has any.
pub(super) fn filed_message(url: &str, handoff: Handoff) -> String {
    let key = match handoff {
        Handoff::None => "bug_report.filed",
        Handoff::Shown => "bug_report.filed_screenshots",
        Handoff::ByHand => "bug_report.filed_screenshots_by_hand",
    };
    knot_core::l10n::t_with(key, &[("url", url)])
}

/// Whether Report can be chosen: both fields hold something other than
/// whitespace, and no submission is already in flight.
pub(super) fn can_report(subject: &str, description: &str, phase: &Phase) -> bool {
    !subject.trim().is_empty() && !description.trim().is_empty() && *phase != Phase::Submitting
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_fields_are_required() {
        assert!(!can_report("", "it broke", &Phase::Editing));
        assert!(!can_report("Crash", "", &Phase::Editing));
        assert!(can_report("Crash", "it broke", &Phase::Editing));
    }

    #[test]
    fn whitespace_is_not_content() {
        assert!(!can_report("  \t", "it broke", &Phase::Editing));
        assert!(!can_report("Crash", "\n  \n", &Phase::Editing));
    }

    #[test]
    fn a_submission_in_flight_blocks_another() {
        assert!(!can_report("Crash", "it broke", &Phase::Submitting));
    }

    #[test]
    fn a_failure_or_a_hand_off_can_be_retried() {
        for phase in [Phase::Failed("403".to_owned()),
                      Phase::BrowserReady(Handoff::None),
                      Phase::BrowserFailed]
        {
            assert!(can_report("Crash", "it broke", &phase), "{phase:?}");
        }
    }

    #[test]
    fn every_status_message_resolves() {
        for phase in [Phase::Submitting,
                      Phase::Failed("403".to_owned()),
                      Phase::BrowserReady(Handoff::None),
                      Phase::BrowserReady(Handoff::Shown),
                      Phase::BrowserReady(Handoff::ByHand),
                      Phase::BrowserFailed]
        {
            let message = phase.message()
                               .expect("every phase but Editing has a message");
            assert!(!message.starts_with("bug_report."),
                    "{phase:?} rendered its key");
        }
        assert_eq!(Phase::Editing.message(), None);
    }

    #[test]
    fn a_failure_names_its_cause() {
        let message = Phase::Failed("HTTP 403".to_owned()).message().unwrap();
        assert!(message.contains("HTTP 403"), "{message}");
    }

    /// Each hand-off has its own confirmation, and every one keeps the URL.
    #[test]
    fn the_filed_message_follows_the_handoff() {
        let url = "https://github.com/pilgrimagesoftware/Knot/issues/7";
        let messages = [Handoff::None, Handoff::Shown, Handoff::ByHand].map(|handoff| {
                                                                           filed_message(url,
                                                                                         handoff)
                                                                       });
        for message in &messages {
            assert!(message.contains(url), "the URL was lost: {message}");
            assert!(!message.contains("%{"), "a placeholder was left: {message}");
        }
        assert_ne!(messages[0], messages[1]);
        assert_ne!(messages[1], messages[2]);
    }
}
