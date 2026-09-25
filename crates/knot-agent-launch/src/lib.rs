//! Decides how to launch an agent - a shell agent's terminal command, or a
//! non-shell agent's ACP adapter - and builds the registration prompts sent
//! to it, and the startup prompt that follows them.
//!
//! Contract: `openspec/specs/agent-launch-command/spec.md` and
//! `openspec/specs/prompt-library/spec.md`.

mod adapter;
mod builders;
mod capabilities;
mod consts;
mod escape;
mod registration;
mod startup;
mod variables;

pub use adapter::{AdapterConfig, InstallMethod, acp_adapter, adapter_path, adapter_path_for};
pub use builders::{
    LaunchPlan, LaunchRequest, build_agent_command, build_initialization_command, mcp_url,
    mcp_url_for_port, plan_launch,
};
pub use capabilities::supports_inline_registration;
pub use escape::persona_prompt;
pub use registration::{
    acp_registration_prompt, knot_instructions, registration_prompt, registration_user_prompt,
};
pub use startup::{ContextSource, read_context, resolve_startup_prompt};
pub use variables::{PromptContext, PromptVariable, UnknownVariable, expand, unknown_variables};
