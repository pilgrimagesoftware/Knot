//! Composing the one list the section shows out of two sources.
//!
//! Knot's own server and the agent's own servers answer to different
//! authorities and must not be flattened into one shape. Knot's comes from
//! its own supervisor - instant, authoritative, and carrying a richer state
//! than any probe could report (an address while running, an attempt count
//! and an error while retrying). The agent's come from a probe of a separate
//! process, and are stamped with when they were taken.
//!
//! Keeping them as two variants of [`SectionRow`] rather than mapping Knot's
//! state onto [`knot_mcp_probe::ServerState`] does two things. It keeps
//! `Starting` and `Retrying { attempt, error }` - which have no probe
//! equivalent and would land on `Unknown` - and it makes "Knot's row offers
//! no delegated action" a property of the type rather than a runtime check
//! someone can forget.

use knot_mcp_probe::{Inventory, ServerRow, same_endpoint};

/// Knot's own MCP server as the section shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KnotRow {
    pub(crate) state:           knot_mcp::ServerState,
    pub(crate) url:             String,
    /// The agent's own configuration registers this endpoint too.
    ///
    /// Worth saying out loud rather than merging silently: that copy was
    /// added by hand, it outlives the session Knot controls, and removing it
    /// is the user's to do.
    pub(crate) also_configured: bool,
}

/// One row of the section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SectionRow {
    Knot(KnotRow),
    Agent(ServerRow),
}

impl SectionRow {
    /// Whether this row offers the delegated action.
    ///
    /// Never for Knot's own: its lifecycle is Knot's own - supervised and
    /// restarted here, configured in the settings window's MCP tab - so
    /// there is no agent flow to hand the user over to, in any state
    /// including failed.
    // UNWIRED(#383): the row's delegated action button, task group 7.
    #[allow(dead_code)]
    pub(crate) fn offers_action(&self) -> bool {
        match self {
            Self::Knot(_) => false,
            Self::Agent(row) => row.state.offers_action(),
        }
    }
}

/// The section's rows: Knot's own first, then the agent's own.
///
/// `inventory` is `None` when no probe has completed - which is not the same
/// as an empty one, and neither is the same as [`Inventory::Unprobeable`].
/// All three show Knot's row; they differ in what the header says beneath it.
pub(crate) fn compose(knot_state: knot_mcp::ServerState, knot_url: &str,
                      inventory: Option<&Inventory>)
                      -> Vec<SectionRow> {
    let mut knot = KnotRow { state:           knot_state,
                             url:             knot_url.to_owned(),
                             also_configured: false, };

    let mut rows = Vec::new();

    for row in inventory.map(Inventory::rows).unwrap_or_default() {
        // Identity is the endpoint, not the name. `MCP_SERVER_NAME` is only
        // what Knot registers under; a user who ran the settings window's
        // install command by hand chose their own, and a server named like
        // Knot's but pointing elsewhere is somebody else's entirely.
        if same_endpoint(row.target.full(), knot_url) {
            knot.also_configured = true;
            continue;
        }

        rows.push(SectionRow::Agent(row.clone()));
    }

    rows.insert(0, SectionRow::Knot(knot));

    rows
}

#[cfg(test)]
mod tests;
