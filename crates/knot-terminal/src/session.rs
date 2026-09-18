use std::path::Path;

use knot_agent_launch::{LaunchRequest, build_agent_command, build_initialization_command};
use knot_agents::Agent;
use knot_core::{Persona, Settings};

use crate::Result;

pub trait TerminalTransport: Send {
    fn send_text(&mut self, text: &str) -> Result<()>;
    fn send_return(&mut self) -> Result<()>;
    fn terminate(&mut self) -> Result<()>;

    fn resize(&mut self, _rows: u16, _cols: u16) -> Result<()> {
        Ok(())
    }
}

pub struct SessionConfig<'a> {
    pub settings:    &'a Settings,
    pub agent:       &'a Agent,
    pub persona:     Option<&'a Persona>,
    pub plugin_root: Option<&'a Path>,
}

pub struct SessionPlan {
    pub agent_command:          String,
    pub initialization_command: String,
}

impl SessionPlan {
    pub fn build(config: &SessionConfig<'_>) -> Self {
        let request = LaunchRequest { agent_type:        &config.agent.agent_type,
                                      agent_id:          Some(config.agent.id),
                                      shell_command:     config.agent.shell_command.as_deref(),
                                      resume_session_id: config.agent.resume_session_id.as_deref(),
                                      fork_session:      config.agent.fork_session,
                                      persona:           config.persona,
                                      plugin_root:       config.plugin_root, };
        let agent_command = build_agent_command(config.settings, &request);
        let initialization_command = build_initialization_command(&config.agent.folder,
                                                                  &agent_command,
                                                                  Some(config.agent.id));
        Self { agent_command,
               initialization_command }
    }
}
