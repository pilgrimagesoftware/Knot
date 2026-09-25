//! Building the command that hands the user an agent's own MCP flow.
//!
//! Contract: `openspec/specs/agent-mcp-status/spec.md` - "A server needing
//! attention delegates to the agent's own flow".
//!
//! Everything here is assembled from `knot_core::agent_type`'s roster, with
//! exactly one exception: the server's name, for a type whose command
//! addresses one server directly. That name comes from another program's
//! output and ends up on a command line that Knot *types into a terminal*,
//! which is why it is both validated and quoted.
//!
//! The danger is not hypothetical and it is not the usual one. Single quotes
//! already neutralize `;`, `$(` and backticks. What they do not neutralize is
//! a newline: the text is delivered to a PTY followed by Return, so a newline
//! inside the name submits the line early and whatever follows runs as its
//! own command. That is why control characters are rejected outright rather
//! than escaped.

use knot_core::agent_type::McpManage;

/// The longest server name that may be substituted into a command.
///
/// Generous next to any real name and short enough that a pathological one
/// cannot fill a terminal line.
const MAX_NAME: usize = 128;

/// What to do in the terminal that was opened for the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Handover {
    /// Run this command line. It addresses one named server directly, so
    /// there is nothing left for the user to navigate.
    Command { command: String },
    /// Run this command line, then send `send` to it - the agent's own
    /// interactive MCP UI, which names no server.
    Interactive { command: String, send: String },
}

impl Handover {
    /// The command line to run.
    pub(crate) fn command(&self) -> &str {
        match self {
            Self::Command { command } | Self::Interactive { command, .. } => command,
        }
    }

    /// What to send once it is running, if anything.
    pub(crate) fn send(&self) -> Option<&str> {
        match self {
            Self::Command { .. } => None,
            Self::Interactive { send, .. } => Some(send),
        }
    }
}

/// What to run to hand the user `agent_type`'s own MCP flow for `server`.
///
/// `program` is the binary the agent actually runs - the user's configured
/// command when they set one, so the flow opens in the same installation the
/// probe read from.
///
/// Returns `None` for a type with no MCP command at all, which is how a row
/// on such a type ends up offering no action.
pub(crate) fn handover(agent_type: &str, program: &str, server: &str) -> Option<Handover> {
    match knot_core::agent_type::mcp_manage(agent_type) {
        McpManage::None => None,

        McpManage::Interactive { args, send } => {
            Some(Handover::Interactive { command: join(program, args),
                                         send:    send.to_owned(), })
        }

        McpManage::PerServer(args) => {
            // A name we will not put on a command line is not a reason to
            // offer nothing: the interactive flow reaches the same place, it
            // just makes the user pick the server themselves.
            if !is_safe_name(server) {
                return interactive_fallback(agent_type, program);
            }

            let quoted = shell_quote(server);
            let substituted: Vec<String> = args.iter()
                                               .map(|arg| arg.replace("%{server}", &quoted))
                                               .collect();

            Some(Handover::Command { command: join_owned(program, &substituted), })
        }
    }
}

/// What to do for a type whose per-server command could not be built.
///
/// A type is recorded with *one* handover shape, so a `PerServer` type has no
/// interactive command string to fall back to. Its CLI itself is the next
/// best thing: the user lands in the agent, in the right folder, one step
/// further from the server than they would have been. That step is the cost
/// of not typing a name we are unwilling to quote, and it beats both offering
/// nothing and guessing at a subcommand.
fn interactive_fallback(agent_type: &str, program: &str) -> Option<Handover> {
    match knot_core::agent_type::mcp_manage(agent_type) {
        McpManage::Interactive { args, send } => {
            Some(Handover::Interactive { command: join(program, args),
                                         send:    send.to_owned(), })
        }
        McpManage::PerServer(_) => Some(Handover::Command { command: program.to_owned(), }),
        McpManage::None => None,
    }
}

/// Whether a name from foreign output may be substituted into a command.
///
/// Deliberately about *characters*, not about a shape: real server names hold
/// spaces, dots, colons and parentheses (`claude.ai Asana (2)`,
/// `plugin:kochava:github`), and a pattern tight enough to be reassuring
/// would reject most of them. Quoting handles the shell; this handles what
/// quoting cannot.
fn is_safe_name(name: &str) -> bool {
    !name.is_empty() && name.chars().count() <= MAX_NAME && !name.chars().any(char::is_control)
}

/// Wraps `text` in single quotes, ending and reopening them around any single
/// quote it contains - the only form that is total for POSIX shells.
fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

fn join(program: &str, args: &[&str]) -> String {
    if args.is_empty() {
        return program.to_owned();
    }

    format!("{program} {}", args.join(" "))
}

fn join_owned(program: &str, args: &[String]) -> String {
    if args.is_empty() {
        return program.to_owned();
    }

    format!("{program} {}", args.join(" "))
}

#[cfg(test)]
mod tests;
