//! What kind of issue a report files: a bug, or a feature request (#518).
//!
//! The kind decides the GitHub label, the fields' placeholders, and whether
//! the logs are offered - a feature request has nothing for a log to explain.

use std::fmt;
use std::str::FromStr;

/// The kinds of issue the dialog can file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum IssueKind {
    #[default]
    Bug,
    FeatureRequest,
}

impl IssueKind {
    /// In the order the dialog offers them.
    pub(crate) const ALL: [Self; 2] = [Self::Bug, Self::FeatureRequest];

    /// The repository label the issue is filed under. Both are GitHub's
    /// default labels, which the repository carries.
    pub(crate) fn github_label(self) -> &'static str {
        match self {
            Self::Bug => "bug",
            Self::FeatureRequest => "enhancement",
        }
    }

    /// The choice's name in the dialog.
    pub(crate) fn label(self) -> String {
        knot_core::l10n::t(match self {
                               Self::Bug => "bug_report.kind.bug",
                               Self::FeatureRequest => "bug_report.kind.feature",
                           })
    }

    pub(crate) fn subject_placeholder(self) -> String {
        knot_core::l10n::t(match self {
                               Self::Bug => "bug_report.subject_placeholder",
                               Self::FeatureRequest => "bug_report.feature_subject_placeholder",
                           })
    }

    pub(crate) fn description_placeholder(self) -> String {
        knot_core::l10n::t(match self {
                               Self::Bug => "bug_report.description_placeholder",
                               Self::FeatureRequest => "bug_report.feature_description_placeholder",
                           })
    }

    /// Whether the logs are offered. A feature request carries the
    /// diagnostics - the version still matters - but no log.
    pub(crate) fn offers_logs(self) -> bool {
        self == Self::Bug
    }
}

impl fmt::Display for IssueKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
                        Self::Bug => "bug",
                        Self::FeatureRequest => "feature",
                    })
    }
}

impl FromStr for IssueKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "bug" => Ok(Self::Bug),
            "feature" => Ok(Self::FeatureRequest),
            other => Err(format!("unknown issue kind: {other}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_round_trips() {
        for kind in IssueKind::ALL {
            assert_eq!(kind.to_string().parse::<IssueKind>(), Ok(kind));
        }
        assert!("question".parse::<IssueKind>().is_err());
    }

    #[test]
    fn each_kind_files_under_its_own_label() {
        assert_eq!(IssueKind::Bug.github_label(), "bug");
        assert_eq!(IssueKind::FeatureRequest.github_label(), "enhancement");
    }

    #[test]
    fn only_a_bug_offers_the_logs() {
        assert!(IssueKind::Bug.offers_logs());
        assert!(!IssueKind::FeatureRequest.offers_logs());
    }

    #[test]
    fn every_kind_string_resolves() {
        for kind in IssueKind::ALL {
            for text in [kind.label(),
                         kind.subject_placeholder(),
                         kind.description_placeholder()]
            {
                assert!(!text.starts_with("bug_report."), "{kind:?}: {text}");
            }
        }
    }
}
