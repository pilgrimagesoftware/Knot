//! Decides how to launch an agent - a shell agent's terminal command, or a
//! non-shell agent's ACP adapter - and builds the registration prompts sent
//! to it.
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
    LaunchPlan, LaunchRequest, build_agent_command, build_initialization_command, mcp_url,
    plan_launch,
};
pub use capabilities::supports_inline_registration;
pub use escape::persona_prompt;
pub use registration::{
    acp_registration_prompt, knot_instructions, registration_prompt, registration_user_prompt,
};
