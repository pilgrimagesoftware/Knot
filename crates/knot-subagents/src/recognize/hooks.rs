//! Reading a posted hook event for a subagent's lifecycle.
//!
//! Contract: `openspec/specs/agent-hooks/spec.md` - "Subagent lifecycle hook
//! events".
//!
//! The terminal-mode feed. An agent running in a pseudo-terminal has no
//! protocol stream to read, so it posts to the hook route Knot already serves;
//! this turns one such body into the same [`SubagentEvent`] the ACP path
//! produces.
//!
//! ## Naming
//!
//! The `hook` values are PascalCase because they are Claude Code's own event
//! names, forwarded by the plugin rather than invented here. The payload keys
//! are snake_case, matching the rest of the hook surface (`transcript_path`,
//! `session_id`).
//!
//! ## Nothing here emits these yet
//!
//! The plugin that posts them lives outside this repo, which is why `claude`
//! is `SubagentReporting::ToolCalls` on the roster rather than `Either`. This
//! code is the contract Knot accepts, written so that the emitter has
//! something to be written against - not dead code waiting for a caller: the
//! route dispatches to it the moment an event arrives.

use serde_json::Value;

use crate::event::SubagentEvent;

#[cfg(test)]
mod tests;

/// A subagent was dispatched.
pub const HOOK_SUBAGENT_START: &str = "SubagentStart";
/// A subagent finished, one way or another.
pub const HOOK_SUBAGENT_STOP: &str = "SubagentStop";

/// Whether `hook` names one of the subagent lifecycle events.
///
/// The route asks this before anything else: a subagent event must not move
/// the agent's activity status, so it has to be told apart from a status post
/// before either is handled.
#[must_use]
pub fn is_subagent_hook(hook: &str) -> bool {
    hook == HOOK_SUBAGENT_START || hook == HOOK_SUBAGENT_STOP
}

/// The event this hook body reports, or `None` if it reports none.
///
/// `None` covers both "not a subagent hook" and "a subagent hook whose payload
/// cannot supply an event". The route distinguishes them: a malformed subagent
/// payload is a 400, per the spec, while an unknown hook is simply not this
/// feed's business.
#[must_use]
pub fn recognize_hook(hook: &str, payload: &Value) -> Option<SubagentEvent> {
    let id = string_field(payload, "subagent_id")?;

    match hook {
        HOOK_SUBAGENT_START => SubagentEvent::dispatched(id,
                                                         string_field(payload, "subagent_type"),
                                                         string_field(payload, "task")),
        HOOK_SUBAGENT_STOP => SubagentEvent::completed(id,
                                                       string_field(payload, "outcome")?,
                                                       string_field(payload, "reason")),
        _ => None,
    }
}

/// Whether a subagent hook body carries the fields its event needs.
///
/// Separate from [`recognize_hook`] because the route owes the poster a 400
/// for a malformed payload, and `None` alone cannot say whether the payload
/// was wrong or the hook was simply not ours.
#[must_use]
pub fn subagent_payload_is_complete(hook: &str, payload: &Value) -> bool {
    match hook {
        HOOK_SUBAGENT_START => {
            string_field(payload, "subagent_id").is_some()
            && string_field(payload, "task").is_some()
        }
        HOOK_SUBAGENT_STOP => {
            string_field(payload, "subagent_id").is_some()
            && string_field(payload, "outcome").is_some()
        }
        _ => false,
    }
}

/// A non-blank string field, or `None`. Blank is absent: a posted `""` is the
/// emitter having nothing to say, not a value.
fn string_field<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload.get(key)?
           .as_str()
           .map(str::trim)
           .filter(|value| !value.is_empty())
}
