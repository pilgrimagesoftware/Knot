use knot_core::Settings;

use crate::adapter::AdapterConfig;

#[derive(Debug, Clone, Default)]
pub struct LaunchRequest<'a> {
    pub agent_type:    &'a str,
    pub shell_command: Option<&'a str>,
}

/// The knot MCP HTTP server's own URL, for wiring into an ACP session's
/// `mcpServers` params.
pub fn mcp_url(settings: &Settings) -> String {
    mcp_url_for_port(settings.mcp_server_port)
}

/// The same URL from a bare port, for callers that have no `Settings` -
/// notably the settings window, which shows the URL and the `mcp add`
/// command a user copies to register Knot with an agent by hand.
///
/// One function, because the path is the part that gets forgotten: the
/// server routes MCP at `/mcp` and answers a POST to `/` with 405, so a
/// URL built without it produces an entry that can never connect.
pub fn mcp_url_for_port(port: u16) -> String {
    format!("http://127.0.0.1:{port}/mcp")
}

/// Builds the shell agent's terminal command: its custom command if set,
/// otherwise empty (a plain interactive shell). Non-shell agent types no
/// longer launch through this builder - see `plan_launch`.
pub fn build_agent_command(request: &LaunchRequest<'_>) -> String {
    request.shell_command.unwrap_or_default().to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchPlan {
    Terminal(String),
    Adapter(AdapterConfig),
}

/// Decides how to launch an agent: a shell agent always gets the Terminal
/// path; every other type launches through its registered ACP adapter (see
/// `acp_adapter`) - there is no Terminal fallback for non-shell agents.
pub fn plan_launch(request: &LaunchRequest<'_>) -> LaunchPlan {
    if !knot_core::agent_type::is_shell(request.agent_type)
       && let Some(config) = crate::adapter::acp_adapter(request.agent_type)
    {
        return LaunchPlan::Adapter(config);
    }
    LaunchPlan::Terminal(build_agent_command(request))
}

/// Build the terminal initialization command: `cd` into `folder`, clear the
/// screen, then (for a non-empty `agent_command`) run it. The leading space
/// suppresses shell history under `ignorespace`. Shell agents only - no
/// `KNOT_AGENT_ID` is set (shell agents do not register with MCP).
pub fn build_initialization_command(folder: &str, agent_command: &str) -> String {
    if agent_command.is_empty() {
        return format!(" cd '{folder}' && clear");
    }
    format!(" cd '{folder}' && clear && {agent_command}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_agent_custom_command() {
        let req = LaunchRequest { agent_type:    "shell",
                                  shell_command: Some("htop"), };
        assert_eq!(build_agent_command(&req), "htop");
    }

    #[test]
    fn shell_agent_no_custom_command_is_empty() {
        let req = LaunchRequest { agent_type: "shell",
                                  ..Default::default() };
        assert_eq!(build_agent_command(&req), "");
    }

    #[test]
    fn shell_agent_wrapper_with_no_custom_command() {
        let cmd = build_initialization_command("/tmp/repo", "");
        assert_eq!(cmd, " cd '/tmp/repo' && clear");
    }

    #[test]
    fn shell_agent_wrapper_with_a_custom_command() {
        let cmd = build_initialization_command("/tmp/repo", "htop");
        assert_eq!(cmd, " cd '/tmp/repo' && clear && htop");
    }

    #[test]
    fn shell_agent_always_uses_the_terminal_path() {
        let req = LaunchRequest { agent_type: "shell",
                                  ..Default::default() };
        let plan = plan_launch(&req);
        assert_eq!(plan, LaunchPlan::Terminal(build_agent_command(&req)));
    }

    #[test]
    fn non_shell_agent_with_a_registered_adapter_launches_via_acp() {
        let req = LaunchRequest { agent_type: "claude",
                                  ..Default::default() };
        let plan = plan_launch(&req);
        let LaunchPlan::Adapter(config) = plan
        else {
            panic!("expected an adapter launch plan, not a terminal command");
        };
        assert_eq!(config, crate::adapter::acp_adapter("claude").unwrap());
    }

    #[test]
    fn non_shell_agent_without_a_registered_adapter_has_no_launch_plan_worth_running() {
        let req = LaunchRequest { agent_type: "unknown-type",
                                  ..Default::default() };
        let plan = plan_launch(&req);
        assert_eq!(plan, LaunchPlan::Terminal(build_agent_command(&req)));
    }

    #[test]
    fn mcp_url_uses_the_configured_port() {
        let mut s = Settings::default();
        s.mcp_server_port = 8767;
        assert_eq!(mcp_url(&s), "http://127.0.0.1:8767/mcp");
    }
}
