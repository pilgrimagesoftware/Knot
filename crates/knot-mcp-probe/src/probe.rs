//! Running one probe: the command, the parse, and the inventory it produces.
//!
//! Blocking, like the rest of the crate. Call it from `spawn_blocking`.

use std::path::PathBuf;

use crate::error::Result;
use crate::inventory::Inventory;
use crate::parse::ListFormat;
use crate::runner::{McpRunner, ProbeCommand};

/// What to run for one agent, or that there is nothing to run.
///
/// [`ProbePlan::Unsupported`] is a plan, not an absence: it is how an agent
/// type Knot has no way to interrogate reaches the section as "cannot
/// determine" rather than as an empty list. Making the caller choose between
/// two variants is what stops that distinction being lost at the call site,
/// which an `Option<ProbeCommand>` would invite.
#[derive(Debug, Clone)]
pub enum ProbePlan {
    /// This agent type has no MCP list command.
    Unsupported,
    Run {
        command: ProbeCommand,
        format:  ListFormat,
    },
}

/// What to run for one agent, from its type and where it is running.
///
/// `program_override` is the command the user configured for this agent type
/// in settings, if any. Its first token is taken as the binary: the setting
/// may carry flags meant for launching the agent (`claude --resume`), and
/// those are not arguments to `mcp list`. Using it at all matters because a
/// user who points Knot at a particular build of a CLI means that build - a
/// probe that ran a different one on `PATH` would be reading a different
/// installation's configuration.
#[must_use]
pub fn plan_for(agent_type: &str, cwd: impl Into<PathBuf>, env: Vec<(String, String)>,
                program_override: Option<&str>)
                -> ProbePlan {
    let (Some(command), Some(format)) = (knot_core::agent_type::mcp_list_command(agent_type),
                                         ListFormat::for_agent_type(agent_type))
    else {
        return ProbePlan::Unsupported;
    };

    let (default_program, args) = command.split_first().expect("a non-empty list command");

    let program = program_override.and_then(|text| text.split_whitespace().next())
                                  .unwrap_or(default_program);

    let command = ProbeCommand::new(program,
                                    args.iter().map(|arg| (*arg).to_owned()).collect(),
                                    cwd).with_env(env);

    ProbePlan::Run { command, format }
}

/// Asks an agent which MCP servers it has.
///
/// # Errors
///
/// Any [`crate::ProbeError`]: the command was missing, timed out, failed with
/// nothing to show, or produced output in a shape the parser does not know.
/// Every one of those is something the section reports in words; none of them
/// is an empty inventory.
pub fn probe(runner: &dyn McpRunner, plan: &ProbePlan) -> Result<Inventory> {
    let ProbePlan::Run { command, format } = plan
    else {
        return Ok(Inventory::Unprobeable);
    };

    let output = runner.run(command)?;
    let rows = format.read(&command.program, &output)?;

    Ok(Inventory::probed(rows))
}

#[cfg(test)]
mod tests;
