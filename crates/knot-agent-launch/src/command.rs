//! `LaunchRequest` and the two top-level builders: the full agent command
//! and the terminal initialization wrapper.

use std::path::Path;

use knot_core::{Persona, Settings, ViewMode};
use uuid::Uuid;

use crate::adapter::AdapterConfig;
use crate::capabilities::{can_fork, can_resume, supports_inline_registration};
use crate::registration::{inline_registration_arguments, mcp_arguments};

/// Per-launch inputs to [`build_agent_command`]. Everything durable
/// (commands, options, MCP config) comes from `Settings` instead.
#[derive(Debug, Clone, Default)]
pub struct LaunchRequest<'a> {
    pub agent_type:        &'a str,
    pub agent_id:          Option<Uuid>,
    pub shell_command:     Option<&'a str>,
    pub resume_session_id: Option<&'a str>,
    pub fork_session:      bool,
    pub persona:           Option<&'a Persona>,
    pub plugin_root:       Option<&'a Path>,
}

/// The configured base command for `agent_type`: the stored override if
/// one is set, otherwise the agent type name itself for predefined types
/// (`custom1`/`custom2` default to empty - they have no built-in binary).
fn base_command(settings: &Settings, agent_type: &str) -> String {
    if let Some(cmd) = settings.agent_commands.get(agent_type) {
        return cmd.clone();
    }
    match agent_type {
        "custom1" | "custom2" => String::new(),
        _ => agent_type.to_string(),
    }
}

fn mcp_url(settings: &Settings) -> String {
    format!("http://127.0.0.1:{}/mcp", settings.mcp_server_port)
}

/// Build the full agent command: base command, resume/fork arguments, user
/// options, and (when MCP is enabled) MCP configuration plus inline
/// registration arguments. Per
/// `openspec/specs/agent-launch-command/spec.md`.
pub fn build_agent_command(settings: &Settings, request: &LaunchRequest<'_>) -> String {
    if request.agent_type == "shell" {
        return request.shell_command.unwrap_or_default().to_string();
    }

    let cmd = base_command(settings, request.agent_type);
    if cmd.is_empty() {
        return String::new();
    }

    let mut full = cmd;

    if let Some(session_id) = request.resume_session_id
       && can_resume(request.agent_type)
    {
        let fork = request.fork_session && can_fork(request.agent_type);
        match request.agent_type {
            "codex" => {
                if fork {
                    full.push_str(&format!(" fork {session_id}"));
                }
                else {
                    full.push_str(&format!(" resume {session_id}"));
                }
            }
            _ => {
                full.push_str(&format!(" --resume {session_id}"));
                if fork {
                    full.push_str(" --fork-session");
                }
            }
        }
    }

    if let Some(opts) = settings.agent_options.get(request.agent_type)
       && !opts.is_empty()
    {
        full.push(' ');
        full.push_str(opts);
    }

    full.push_str(&mcp_and_registration_args(settings, request));

    full
}

/// MCP configuration plus inline-registration arguments for `request`, or
/// an empty string when MCP is disabled - the single place both the
/// terminal command and the adapter's own configuration mechanism
/// (`plan_launch`'s `LaunchPlan::Adapter`) get this from, so the two launch
/// paths can't drift apart for the same settings.
fn mcp_and_registration_args(settings: &Settings, request: &LaunchRequest<'_>) -> String {
    if !settings.mcp_server_enabled {
        return String::new();
    }
    let mut args = mcp_arguments(request.agent_type, &mcp_url(settings), request.plugin_root);
    if let Some(agent_id) = request.agent_id
       && supports_inline_registration(request.agent_type)
    {
        args.push_str(&inline_registration_arguments(request.agent_type,
                                                     agent_id,
                                                     request.resume_session_id.is_some(),
                                                     request.persona));
    }
    args
}

/// An ACP adapter launch: the registered adapter plus the MCP/registration
/// configuration to pass through its own configuration mechanism, matching
/// the terminal path's MCP-enabled and MCP-disabled behavior per
/// `openspec/specs/agent-launch-command/spec.md`'s "Adapter-carried MCP and
/// registration" requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterLaunch {
    pub config:     AdapterConfig,
    pub mcp_config: String,
}

/// The decided way to launch an agent: the existing terminal command, or an
/// ACP adapter subprocess to connect to via `knot-acp`. Per
/// `openspec/specs/agent-launch-command/spec.md`'s "ACP launch path"
/// requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchPlan {
    Terminal(String),
    Adapter(AdapterLaunch),
}

/// Decides between the terminal and ACP adapter launch paths: Panel mode
/// with a registered adapter (`adapter`, typically `acp_adapter(request
/// .agent_type)`) uses the adapter; Terminal mode or no registered adapter
/// falls back to the existing terminal command unchanged.
pub fn plan_launch(view_mode: ViewMode, adapter: Option<AdapterConfig>, settings: &Settings,
                   request: &LaunchRequest<'_>)
                   -> LaunchPlan {
    if view_mode == ViewMode::Panel
       && let Some(config) = adapter
    {
        return LaunchPlan::Adapter(AdapterLaunch { config,
                                                   mcp_config:
                                                       mcp_and_registration_args(settings, request) });
    }
    LaunchPlan::Terminal(build_agent_command(settings, request))
}

/// Build the terminal initialization command: `cd` into `folder`, clear the
/// screen, then (for a non-empty `agent_command`) set `KNOT_AGENT_ID` and
/// run it. The leading space suppresses shell history under `ignorespace`.
pub fn build_initialization_command(folder: &str, agent_command: &str, agent_id: Option<Uuid>)
                                    -> String {
    if agent_command.is_empty() {
        return format!(" cd '{folder}' && clear");
    }
    let env_prefix = agent_id.map(|id| format!("KNOT_AGENT_ID={id} "))
                             .unwrap_or_default();
    format!(" cd '{folder}' && clear && {env_prefix}{agent_command}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Settings {
        Settings::default()
    }

    #[test]
    fn missing_command_yields_nothing() {
        let mut s = settings();
        s.agent_commands.insert("claude".to_string(), String::new());
        let req = LaunchRequest { agent_type: "claude",
                                  ..Default::default() };
        assert_eq!(build_agent_command(&s, &req), "");
    }

    #[test]
    fn default_command_falls_back_to_agent_type() {
        let mut s = settings();
        s.mcp_server_enabled = false;
        let req = LaunchRequest { agent_type: "claude",
                                  ..Default::default() };
        assert_eq!(build_agent_command(&s, &req), "claude");
    }

    #[test]
    fn shell_agent_custom_command() {
        let s = settings();
        let req = LaunchRequest { agent_type: "shell",
                                  shell_command: Some("htop"),
                                  ..Default::default() };
        assert_eq!(build_agent_command(&s, &req), "htop");
    }

    #[test]
    fn shell_agent_no_custom_command_is_empty() {
        let s = settings();
        let req = LaunchRequest { agent_type: "shell",
                                  ..Default::default() };
        assert_eq!(build_agent_command(&s, &req), "");
    }

    #[test]
    fn claude_resume_with_fork() {
        let mut s = settings();
        s.mcp_server_enabled = false;
        let req = LaunchRequest { agent_type: "claude",
                                  resume_session_id: Some("abc123"),
                                  fork_session: true,
                                  ..Default::default() };
        assert_eq!(build_agent_command(&s, &req),
                   "claude --resume abc123 --fork-session");
    }

    #[test]
    fn codex_fork_uses_subcommand() {
        let mut s = settings();
        s.mcp_server_enabled = false;
        let req = LaunchRequest { agent_type: "codex",
                                  resume_session_id: Some("abc123"),
                                  fork_session: true,
                                  ..Default::default() };
        let cmd = build_agent_command(&s, &req);
        assert!(cmd.contains("fork abc123"));
        assert!(!cmd.contains("--resume"));
    }

    #[test]
    fn mcp_disabled_omits_all_mcp_and_registration_args() {
        let mut s = settings();
        s.mcp_server_enabled = false;
        let req = LaunchRequest { agent_type: "claude",
                                  agent_id: Some(Uuid::nil()),
                                  ..Default::default() };
        assert_eq!(build_agent_command(&s, &req), "claude");
    }

    #[test]
    fn mcp_enabled_appends_registration() {
        let s = settings();
        let req = LaunchRequest { agent_type: "claude",
                                  agent_id: Some(Uuid::nil()),
                                  ..Default::default() };
        let cmd = build_agent_command(&s, &req);
        assert!(cmd.contains("--mcp-config"));
        assert!(cmd.contains("--append-system-prompt"));
    }

    #[test]
    fn env_var_precedes_agent_command() {
        let cmd = build_initialization_command("/tmp/repo", "claude", Some(Uuid::nil()));
        assert_eq!(cmd,
                   format!(" cd '/tmp/repo' && clear && KNOT_AGENT_ID={} claude",
                           Uuid::nil()));
    }

    #[test]
    fn shell_agent_wrapper_has_no_env_var() {
        let cmd = build_initialization_command("/tmp/repo", "", None);
        assert_eq!(cmd, " cd '/tmp/repo' && clear");
    }

    fn stub_adapter() -> AdapterConfig {
        AdapterConfig { command:                   "claude-code-acp",
                        args:                      &[],
                        supports_resume:           true,
                        supports_permission_modes: true, }
    }

    #[test]
    fn terminal_mode_always_uses_terminal_command_even_with_an_adapter() {
        let s = settings();
        let req = LaunchRequest { agent_type: "claude",
                                  ..Default::default() };
        let plan = plan_launch(ViewMode::Terminal, Some(stub_adapter()), &s, &req);
        assert_eq!(plan, LaunchPlan::Terminal(build_agent_command(&s, &req)));
    }

    #[test]
    fn panel_mode_without_an_adapter_falls_back_to_terminal_command() {
        let s = settings();
        let req = LaunchRequest { agent_type: "claude",
                                  ..Default::default() };
        let plan = plan_launch(ViewMode::Panel, None, &s, &req);
        assert_eq!(plan, LaunchPlan::Terminal(build_agent_command(&s, &req)));
    }

    #[test]
    fn panel_mode_with_an_adapter_spawns_the_adapter_instead_of_a_terminal_command() {
        let s = settings();
        let req = LaunchRequest { agent_type: "claude",
                                  ..Default::default() };
        let plan = plan_launch(ViewMode::Panel, Some(stub_adapter()), &s, &req);
        let LaunchPlan::Adapter(adapter_launch) = plan
        else {
            panic!("expected an adapter launch plan, not a terminal command");
        };
        assert_eq!(adapter_launch.config, stub_adapter());
    }

    #[test]
    fn adapter_mcp_config_matches_the_terminal_paths_mcp_arguments() {
        let s = settings();
        let req = LaunchRequest { agent_type: "claude",
                                  agent_id: Some(Uuid::nil()),
                                  ..Default::default() };

        let terminal_cmd = build_agent_command(&s, &req);
        let LaunchPlan::Adapter(adapter_launch) =
            plan_launch(ViewMode::Panel, Some(stub_adapter()), &s, &req)
        else {
            panic!("expected an adapter launch plan");
        };

        assert!(terminal_cmd.ends_with(&adapter_launch.mcp_config));
        assert!(!adapter_launch.mcp_config.is_empty());
    }

    #[test]
    fn adapter_mcp_config_is_empty_when_mcp_disabled() {
        let mut s = settings();
        s.mcp_server_enabled = false;
        let req = LaunchRequest { agent_type: "claude",
                                  agent_id: Some(Uuid::nil()),
                                  ..Default::default() };

        let LaunchPlan::Adapter(adapter_launch) =
            plan_launch(ViewMode::Panel, Some(stub_adapter()), &s, &req)
        else {
            panic!("expected an adapter launch plan");
        };

        assert_eq!(adapter_launch.mcp_config, "");
    }
}
