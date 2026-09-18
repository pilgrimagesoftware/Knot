//! Builds the shell command that launches an agent in its terminal: base
//! command and user options, resume/fork arguments, MCP configuration and
//! hook plugin injection, inline registration arguments, persona injection,
//! and the working-directory initialization wrapper.
//!
//! Contract: `openspec/specs/agent-launch-command/spec.md`.

mod adapter;
mod builders;
mod capabilities;
mod consts;
mod escape;
mod registration;

pub use adapter::{AdapterConfig, InstallMethod, acp_adapter};
pub use builders::{
    AdapterLaunch, LaunchPlan, LaunchRequest, build_agent_command, build_initialization_command,
    plan_launch,
};
pub use capabilities::{
    can_fork, can_resume, supports_inline_registration, supports_system_prompt,
};
pub use escape::{persona_prompt, shell_escape};
pub use registration::{
    acp_registration_prompt, inline_registration_arguments, knot_instructions, mcp_arguments,
    registration_prompt, registration_user_prompt,
};
