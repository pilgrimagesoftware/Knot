//! Running one probe: the command, the parse, and the inventory it produces.
//!
//! Blocking, like the rest of the crate. Call it from `spawn_blocking`.

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
