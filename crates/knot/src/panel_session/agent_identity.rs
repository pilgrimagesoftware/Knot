//! Which ACP agent a Panel-mode session is running, as the user sees it.
//!
//! Read from the `agentInfo` the adapter reported on `initialize` (issue
//! #591). Pure over the slot's state, so the three cases the header and the
//! Agents menu have to tell apart are testable without a window.

use knot_acp::AgentInfo;

/// What Knot can say about the agent behind a Panel-mode session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AgentIdentity {
    /// No live connection: still connecting, failed, or not a Panel-mode
    /// agent at all. There is no `initialize` response to read.
    NotConnected,
    /// Connected, but the agent sent no usable `agentInfo` - ACP only says
    /// it SHOULD.
    Unreported,
    /// What the agent reported.
    Reported(AgentInfo),
}

impl AgentIdentity {
    /// From a connected session's `agentInfo`.
    pub(crate) fn from_info(info: Option<&AgentInfo>) -> Self {
        info.map_or(Self::Unreported, |info| Self::Reported(info.clone()))
    }

    /// One line naming the agent and its version, or saying why it cannot:
    /// the header button's tooltip and the Agent Info dialog's body.
    ///
    /// The name and version are interpolated into one catalog entry rather
    /// than concatenated here, so a translator controls their order.
    pub(crate) fn summary(&self) -> String {
        match self {
            Self::NotConnected => knot_core::l10n::t("panel.agent_info.not_connected"),
            Self::Unreported => knot_core::l10n::t("panel.agent_info.unreported"),
            Self::Reported(info) => match info.version() {
                Some(version) => knot_core::l10n::t_with("panel.agent_info.reported",
                                                         &[("name", info.display_name()),
                                                           ("version", version)]),
                None => knot_core::l10n::t_with("panel.agent_info.reported_no_version",
                                                &[("name", info.display_name())]),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(title: Option<&str>, version: Option<&str>) -> AgentInfo {
        AgentInfo { name:    "claude-agent-acp".to_string(),
                    title:   title.map(str::to_string),
                    version: version.map(str::to_string), }
    }

    #[test]
    fn no_info_from_a_connected_session_is_unreported() {
        assert_eq!(AgentIdentity::from_info(None), AgentIdentity::Unreported);
    }

    /// The reported name and version both reach the line, whatever order
    /// the catalog puts them in.
    #[test]
    fn a_reported_agent_names_itself_and_its_version() {
        let summary =
            AgentIdentity::from_info(Some(&info(Some("Claude Agent"), Some("0.4.2")))).summary();

        assert!(summary.contains("Claude Agent"), "{summary}");
        assert!(summary.contains("0.4.2"), "{summary}");
        assert!(!summary.contains("%{"),
                "a placeholder went unsubstituted: {summary}");
    }

    /// Without a title the spec says to show `name`; without a version the
    /// line still names the agent rather than falling back to "unreported".
    #[test]
    fn a_reported_agent_without_a_title_or_version_still_names_itself() {
        let summary = AgentIdentity::from_info(Some(&info(None, None))).summary();

        assert!(summary.contains("claude-agent-acp"), "{summary}");
        assert!(!summary.contains("%{"),
                "a placeholder went unsubstituted: {summary}");
    }

    #[test]
    fn every_agent_info_key_resolves() {
        for key in ["panel.agent_info.title",
                    "panel.agent_info.reported",
                    "panel.agent_info.reported_no_version",
                    "panel.agent_info.unreported",
                    "panel.agent_info.not_connected",
                    "menu.agent.agent_info"]
        {
            assert_ne!(knot_core::l10n::t(key), key, "{key} does not resolve");
        }
    }
}
