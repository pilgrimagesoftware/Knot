//! One agent's MCP section, and the claim that keeps a probe from being
//! asked for twice.
//!
//! The section deliberately does *not* store "a probe is running". That is
//! read from the in-flight set, which a [`ProbeClaim`] leaves on drop, so an
//! unwind inside the task cannot strand the header on "checking" for the life
//! of the window. `agent_processes::SamplingClaim` learned that the hard way
//! (issue #376) and this is the same discipline applied per agent.

use std::collections::BTreeSet;
use std::sync::Arc;

use knot_mcp_probe::{Inventory, ProbeError};
use parking_lot::Mutex;
use uuid::Uuid;

/// Agents with a probe in flight.
pub(crate) type InFlight = Arc<Mutex<BTreeSet<Uuid>>>;

/// Where a finished probe reports, and the main thread drains.
pub(crate) type ProbeResults = Arc<Mutex<Vec<(Uuid, Result<Inventory, ProbeError>)>>>;

/// A claim on one agent's probe, released when it drops.
///
/// Also guarantees the section gets an *answer*: if the task unwinds without
/// reporting, the drop reports the failure itself. Without that the section
/// would sit with no rows, no error and no probe running - a state whose
/// header says "not checked yet" while nothing will ever check again.
pub(crate) struct ProbeClaim {
    agent:     Uuid,
    in_flight: InFlight,
    results:   ProbeResults,
    reported:  bool,
}

impl ProbeClaim {
    /// Claims this agent's probe, or `None` when one is already running.
    ///
    /// A request arriving while one is in flight is dropped rather than
    /// queued: the running probe's result answers both.
    pub(crate) fn claim(in_flight: &InFlight, results: &ProbeResults, agent: Uuid) -> Option<Self> {
        in_flight.lock().insert(agent).then(|| Self { agent,
                                                      in_flight: Arc::clone(in_flight),
                                                      results: Arc::clone(results),
                                                      reported: false })
    }

    /// Reports this probe's outcome, consuming the claim.
    pub(crate) fn report(mut self, outcome: Result<Inventory, ProbeError>) {
        self.results.lock().push((self.agent, outcome));
        self.reported = true;
    }
}

impl Drop for ProbeClaim {
    fn drop(&mut self) {
        if !self.reported {
            let unfinished = ProbeError::Io { program: String::new(),
                                              message: "the probe did not finish".to_owned(), };
            self.results.lock().push((self.agent, Err(unfinished)));
        }

        self.in_flight.lock().remove(&self.agent);
    }
}

/// One agent's MCP section.
#[derive(Debug, Default)]
pub(crate) struct McpSection {
    /// Whether the section is open. Does not gate probing: the collapsed
    /// header names what needs attention, so it needs an answer too.
    // UNWIRED(#383): read by the section's render and actions, task groups 5-7.
    #[allow(dead_code)]
    pub(crate) expanded: bool,
    /// A probe is wanted and has not been started.
    requested:           bool,
    /// A probe has been asked for at least once since this section appeared,
    /// so becoming visible again does not re-ask.
    ever_requested:      bool,
    /// The last inventory that landed. Kept across a later failure, so a
    /// probe that fails does not blank rows the user could still act on.
    inventory:           Option<Inventory>,
    /// The most recent failure, shown beside whatever rows survived it.
    failure:             Option<String>,
}

impl McpSection {
    /// Opens or shuts the section, answering its new state.
    // UNWIRED(#383): read by the section's render and actions, task groups 5-7.
    #[allow(dead_code)]
    pub(crate) fn toggle(&mut self) -> bool {
        self.expanded = !self.expanded;
        self.expanded
    }

    /// Asks for a probe: on first becoming visible, on refresh, and when a
    /// delegated terminal exits.
    pub(crate) fn request(&mut self) {
        self.requested = true;
    }

    /// Whether this section has never asked for anything, and so should on
    /// becoming visible.
    pub(crate) fn needs_first_probe(&self) -> bool {
        !self.ever_requested && !self.requested
    }

    /// Takes a pending request, if there is one.
    pub(crate) fn take_request(&mut self) -> bool {
        let requested = std::mem::take(&mut self.requested);
        self.ever_requested |= requested;

        requested
    }

    /// Records an outcome.
    ///
    /// A failure keeps the previous rows deliberately. They carry their own
    /// timestamp, so the section can show both "here is what I last knew" and
    /// "and here is why I could not check again" without pretending either is
    /// the other.
    pub(crate) fn publish(&mut self, outcome: Result<Inventory, ProbeError>) {
        match outcome {
            Ok(inventory) => {
                self.inventory = Some(inventory);
                self.failure = None;
            }
            Err(error) => self.failure = Some(error.to_string()),
        }
    }

    // UNWIRED(#383): read by the section's render and actions, task groups 5-7.
    #[allow(dead_code)]
    pub(crate) fn inventory(&self) -> Option<&Inventory> {
        self.inventory.as_ref()
    }

    // UNWIRED(#383): read by the section's render and actions, task groups 5-7.
    #[allow(dead_code)]
    pub(crate) fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
}

#[cfg(test)]
mod tests;
