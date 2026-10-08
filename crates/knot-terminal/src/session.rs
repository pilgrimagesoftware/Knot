//! What a shell agent's terminal session runs, built from its config.

use std::path::Path;

use knot_agent_launch::{LaunchRequest, build_agent_command, build_initialization_command};
use knot_agents::Agent;
use knot_core::{Persona, Settings};

use crate::AgentShell;

/// `persona`/`plugin_root` are unused now that the terminal path only
/// launches shell agents (they mattered to the removed non-shell
/// registration/persona-injection arguments); kept on the struct so
/// callers built around a full agent config don't need a separate
/// shell-only variant.
pub struct SessionConfig<'a> {
    pub settings:    &'a Settings,
    pub agent:       &'a Agent,
    pub persona:     Option<&'a Persona>,
    pub plugin_root: Option<&'a Path>,
    /// What the PTY runs. [`AgentShell::User`] for every real agent.
    pub shell:       AgentShell,
}

pub struct SessionPlan {
    pub agent_command:          String,
    pub initialization_command: String,
}

impl SessionPlan {
    pub fn build(config: &SessionConfig<'_>) -> Self {
        let request = LaunchRequest { agent_type:    &config.agent.agent_type,
                                      shell_command: config.agent.shell_command.as_deref(), };
        let agent_command = build_agent_command(&request);
        let initialization_command =
            build_initialization_command(&config.agent.folder, &agent_command);
        Self { agent_command,
               initialization_command }
    }
}
