//! Registration prompt text sent to ACP-launched agents (the ACP protocol
//! prompt, not a CLI argument) and to the shell-agent deferred-registration
//! path.

use knot_acp::MCP_SERVER_NAME;
use knot_core::Persona;
use uuid::Uuid;

use crate::consts::REGISTRATION_USER_PROMPT;
use crate::escape::persona_prompt;

/// The user prompt sent on first launch to trigger the agent list table.
pub fn registration_user_prompt() -> &'static str {
    REGISTRATION_USER_PROMPT
}

/// The knot system instructions, with `agent_id` embedded.
///
/// These name the MCP server on purpose. An agent inherits its own
/// user-level MCP configuration on top of the server Knot hands it, and
/// another server there can expose tools with exactly these names -
/// Knot's own Swift predecessor does. The agent then registers with one
/// knot and calls the other, which answers "Agent ID ... not found" for
/// an id it has never seen. Naming the server is what makes that
/// unambiguous, since the tool names alone are not.
pub fn knot_instructions(agent_id: Uuid) -> String {
    format!("You are part of a team of agents called a knot. A knot is made of high-performing agents who collaborate to achieve complex goals so engage with them: ask for help and in return help them succeed. Your knot agent ID: {agent_id}. Your knot's tools come from the MCP server named `{MCP_SERVER_NAME}` (tools such as `{MCP_SERVER_NAME}`'s set-status, list-agents, register-agent). Another MCP server may offer tools with those same names; those belong to a different knot that does not know your agent ID, and calling them will fail or silently do nothing. Only ever use the `{MCP_SERVER_NAME}` server's tools. CRITICAL RULE: Before you start working on anything, your FIRST action must be calling set-status with what you are about to do. When you finish, call set-status again. When you change direction, call set-status. Other agents depend on your status to coordinate — if you do not update it, the team cannot function. This is not optional.")
}

/// The combined registration prompt for the deferred (non-inline)
/// registration path: knot instructions plus an explicit registration
/// request.
pub fn registration_prompt(agent_id: Uuid) -> String {
    format!("{} Register with the knot", knot_instructions(agent_id))
}

/// The ACP protocol registration prompt sent as the first `session/prompt`
/// on a fresh (non-resume) session: knot instructions, persona (if any), and
/// the registration user prompt.
pub fn acp_registration_prompt(agent_id: Uuid, is_resume: bool, persona: Option<&Persona>)
                               -> Option<String> {
    if is_resume {
        return None;
    }
    let mut prompt = knot_instructions(agent_id);
    if let Some(p) = persona_prompt(persona) {
        prompt.push(' ');
        prompt.push_str(&p);
    }
    prompt.push(' ');
    prompt.push_str(registration_user_prompt());
    Some(prompt)
}

#[cfg(test)]
mod tests {
    use knot_core::{PersonaState, PersonaType};

    use super::*;

    fn id() -> Uuid {
        Uuid::nil()
    }

    fn persona(instructions: &str) -> Persona {
        Persona { id:           Uuid::nil(),
                  name:         "Ada".to_string(),
                  instructions: instructions.to_string(),
                  persona_type: PersonaType::User,
                  state:        PersonaState::Enabled, }
    }

    #[test]
    fn registration_prompt_embeds_agent_id_verbatim() {
        let prompt = registration_prompt(id());
        assert!(prompt.contains(&id().to_string()));
        assert!(prompt.ends_with("Register with the knot"));
    }

    /// The agent inherits its own MCP configuration on top of the server
    /// Knot hands it, and another server there can expose identically
    /// named tools - Knot's Swift predecessor does, on a different port.
    /// Without the server name the agent registers with one knot and
    /// queries the other, which answers "Agent ID ... not found".
    #[test]
    fn instructions_name_the_mcp_server_the_tools_come_from() {
        let prompt = knot_instructions(id());
        assert!(prompt.contains(MCP_SERVER_NAME));
        assert!(prompt.contains("Another MCP server may offer tools with those same names"),
                "the prompt must say why the name matters, not just state it");
    }

    #[test]
    fn acp_registration_prompt_fresh_includes_instructions_and_user_prompt() {
        let prompt = acp_registration_prompt(id(), false, None).unwrap();
        assert!(prompt.contains(&id().to_string()));
        assert!(prompt.contains(registration_user_prompt()));
    }

    #[test]
    fn acp_registration_prompt_resume_is_none() {
        assert!(acp_registration_prompt(id(), true, None).is_none());
    }

    #[test]
    fn acp_registration_prompt_includes_persona() {
        let p = persona("Be terse.");
        let prompt = acp_registration_prompt(id(), false, Some(&p)).unwrap();
        assert!(prompt.contains("impersonate Ada"));
    }
}
