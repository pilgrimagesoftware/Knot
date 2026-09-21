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
/// Every sentence is paid for on every launch, in the context window of
/// every agent, so each one has to be something an agent can act on.
/// Restating what a knot is, or that an agent working alone "is just
/// another process", is flattery and simile that change no behavior; the
/// tool names and the moment to reach for each one are the instruction.
/// An earlier version said only "engage with them", named no tools, and
/// produced agents working alone in parallel - which is what a knot
/// exists not to be.
///
/// It pushes outward, not inward: an agent already knows who its knot is,
/// so what it needs telling is to *use* them - hand work to whoever owns
/// that project, ask before guessing at their code, say what it changed
/// that others build on. Instructions to keep checking an inbox would
/// produce busywork instead, and messages reach an agent on their own.
///
/// These name the MCP server on purpose. An agent inherits its own
/// user-level MCP configuration on top of the server Knot hands it, and
/// another server there can expose tools with exactly these names -
/// Knot's own Swift predecessor does. The agent then registers with one
/// knot and calls the other, which answers "Agent ID ... not found" for
/// an id it has never seen. Naming the server is what makes that
/// unambiguous, since the tool names alone are not.
///
/// It stays on one line. The shell-agent path types this prompt into a
/// terminal and then sends Return, so an embedded newline would submit it
/// half-written.
pub fn knot_instructions(agent_id: Uuid) -> String {
    format!("You are part of a team of agents called a knot. Your knot agent ID: {agent_id}. Reach for your knot first and your own effort second. Check list-agents before you start anything substantial: if the work belongs to a teammate's project, hand it to them with send-message rather than working in their code, and ask them rather than reverse-engineer an answer. Use broadcast-message when you change something others build on, and take on what a teammate asks of you. Knot tools come only from the MCP server named `{MCP_SERVER_NAME}` - set-status, list-agents, register-agent and the rest. Another MCP server may offer tools with those same names; those belong to a different knot that does not know your agent ID, and calling them will fail or silently do nothing. CRITICAL: call set-status before you start anything, whenever you change direction, and when you finish. Your teammates coordinate off your status, so this is not optional.")
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

    /// The collaboration tools have to be named, and the moment to use
    /// each one given, or the instruction is a sentiment an agent can obey
    /// by doing nothing. These names are the MCP tool names in
    /// `knot-mcp-tools`; this crate deliberately does not depend on that
    /// one, so a rename there has to be mirrored here.
    #[test]
    fn instructions_name_every_tool_an_agent_collaborates_with() {
        let prompt = knot_instructions(id());
        for tool in ["list-agents",
                     "send-message",
                     "broadcast-message",
                     "set-status"]
        {
            assert!(prompt.contains(tool), "the prompt never mentions {tool}");
        }
    }

    /// The instruction has to point outward - hand work over, ask first -
    /// rather than inward at an inbox. An agent already knows its knot;
    /// what it needs telling is to use them.
    #[test]
    fn instructions_tell_an_agent_to_hand_work_to_its_knot() {
        let prompt = knot_instructions(id());
        assert!(prompt.contains("hand it to them"));
        assert!(prompt.contains("Reach for your knot first"));
    }

    /// The shell-agent path types the prompt into a terminal and then
    /// sends Return, so a newline anywhere in it submits a half-written
    /// prompt and leaves the rest as stray input.
    #[test]
    fn every_registration_prompt_is_a_single_line() {
        assert!(!knot_instructions(id()).contains('\n'));
        assert!(!registration_prompt(id()).contains('\n'));
        assert!(!registration_user_prompt().contains('\n'));
        assert!(!acp_registration_prompt(id(), false, None).unwrap()
                                                           .contains('\n'));
    }

    /// Every launched agent pays for this text in its context window, so
    /// growth is a cost, not a detail. The ceiling is a little above the
    /// current length: adding an instruction is fine, padding it back out
    /// with restatement is what this catches.
    #[test]
    fn instructions_stay_within_their_token_budget() {
        let len = knot_instructions(id()).chars().count();
        assert!(len <= 1_000, "the knot instructions grew to {len} chars");
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
