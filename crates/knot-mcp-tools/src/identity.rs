//! Holding a tool call to the agent it comes from (#539).
//!
//! Contract: `openspec/specs/mcp-tools/spec.md`'s "A caller acts only as
//! itself" requirement.
//!
//! Every tool names its caller in an argument - `from` for the two message
//! senders, `agentId` for the rest - and nothing checked it: a session given
//! another agent's ID read that agent's inbox, set its status and
//! registered as it, while its own ID went unregistered and its messages
//! unread. A connection whose URL names an agent may now speak only for that
//! agent; one that names none may not speak for an agent another live
//! connection holds.

use knot_agents::AgentStore;
use knot_mcp::{Caller, ToolCallResult};
use uuid::Uuid;

use crate::consts;
use crate::lookup::find_by_name_or_id;

/// The argument `tool` names its caller in.
fn caller_argument(tool: &str) -> &'static str {
    match tool {
        consts::SEND_MESSAGE | consts::BROADCAST_MESSAGE => "from",
        _ => "agentId",
    }
}

/// The agent `arguments` say the call comes from, if they name one Knot
/// knows.
pub(crate) fn claimed_agent(store: &AgentStore, tool: &str, arguments: &serde_json::Value)
                            -> Option<Uuid> {
    let claimed = arguments.get(caller_argument(tool))?.as_str()?;
    find_by_name_or_id(store, claimed).map(|agent| agent.id)
}

/// The error to answer `tool` with instead of running it, when its
/// arguments name an agent `caller` may not act as; `None` to run it. A
/// call naming no caller, or one Knot does not know from an unbound
/// connection, is left to the tool, which says what is missing.
pub(crate) fn refusal(store: &AgentStore, tool: &str, arguments: &serde_json::Value,
                      caller: &Caller)
                      -> Option<ToolCallResult> {
    let claimed = arguments.get(caller_argument(tool))?.as_str()?;
    // An ID is its own agent whether or not the store has caught up with it:
    // a connection may arrive before the roster that names it, and refusing
    // an agent its own ID then would lock it out of the knot.
    let named = find_by_name_or_id(store, claimed).map(|agent| agent.id)
                                                  .or_else(|| Uuid::parse_str(claimed).ok());
    match caller.agent {
        Some(bound) if named == Some(bound) => None,
        Some(bound) => Some(ToolCallResult::error(format!(
            "Refused: this connection belongs to {}, but the call names '{claimed}'. \
             An agent can only act as itself. Use the knot agent ID Knot gave you: {bound}.",
            describe(store, bound)
        ))),
        None => {
            let named = named?;
            caller.held_elsewhere(named).then(|| {
                                            ToolCallResult::error(format!(
                    "Refused: {} is already registered from another live session, and two \
                     sessions cannot share one agent ID. Check the knot agent ID Knot gave you; \
                     if you are unsure which agent you are, ask the user.",
                    describe(store, named)
                ))
                                        })
        }
    }
}

/// `agent_id` as the errors name it: `'Name' (id)`, or the id alone for an
/// agent no longer in the knot.
fn describe(store: &AgentStore, agent_id: Uuid) -> String {
    store.agent(agent_id).map_or_else(|| agent_id.to_string(),
                                      |agent| format!("'{}' ({agent_id})", agent.name))
}

#[cfg(test)]
mod tests;
