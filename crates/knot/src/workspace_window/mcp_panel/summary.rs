//! What the MCP section's header says beside its label.
//!
//! Contract: `openspec/specs/agent-mcp-status/spec.md` - "The collapsed
//! section names what needs attention".
//!
//! Collapsed, the header names the servers asking for something, which is the
//! thing worth knowing without opening anything. Expanded, it counts the rows
//! below, because those rows are already the detail.
//!
//! The branch order matters and is not arbitrary. A stopped agent settles the
//! question before any snapshot is consulted; a probe in flight outranks a
//! stale answer; and a failure outranks the rows it failed to refresh, while
//! leaving them on screen - they carry their own timestamp, so showing both
//! "here is what I last knew" and "and here is why I could not check again"
//! tells the truth that showing either alone would not.

use knot_mcp_probe::{Inventory, ServerRow, ServerState};

/// How many names the collapsed header lists before summarizing the rest as
/// a remainder. Three fits beside the label at the narrowest pane width.
const MAX_NAMES: usize = 3;

/// Everything the header's text depends on.
///
/// A struct rather than six arguments, and grouping them keeps the branch
/// order honest: every field is consulted in one place, in one order, rather
/// than by whichever call site happens to know which.
pub(crate) struct SectionStatus<'a> {
    /// The agent has a live session. Settles the question on its own.
    pub(crate) is_running: bool,
    pub(crate) expanded:   bool,
    /// A probe is running now.
    pub(crate) probing:    bool,
    /// The last completed probe, if any.
    pub(crate) inventory:  Option<&'a Inventory>,
    /// The most recent failure, if the last probe failed.
    pub(crate) failure:    Option<&'a str>,
    /// How many rows the expanded section lists - Knot's own included, since
    /// the header counts what is below it and Knot's row is one of them.
    pub(crate) row_count:  usize,
}

/// The header's summary.
pub(crate) fn summary_text(status: &SectionStatus<'_>) -> String {
    if !status.is_running {
        return knot_core::l10n::t("mcp.summary_not_running");
    }

    if status.probing {
        return knot_core::l10n::t("mcp.summary_checking");
    }

    // Ahead of the rows deliberately: the failure is the news, and the rows
    // it could not refresh are still drawn below with their own timestamp.
    if let Some(error) = status.failure {
        return knot_core::l10n::t_with("mcp.summary_failed", &[("error", error)]);
    }

    let Some(inventory) = status.inventory
    else {
        return knot_core::l10n::t("mcp.summary_not_checked");
    };

    // Not "no servers": Knot has no way to ask this agent type. Saying the
    // first would be a confident wrong answer.
    if inventory.is_unprobeable() {
        return knot_core::l10n::t("mcp.summary_cannot_determine");
    }

    if status.expanded {
        return knot_core::l10n::pluralize(status.row_count as u64,
                                          "mcp.count_one",
                                          "mcp.count_many");
    }

    collapsed_text(inventory)
}

/// The collapsed header: what needs attention, or that nothing does.
fn collapsed_text(inventory: &Inventory) -> String {
    if inventory.attention_count() > 0 {
        return attention_text(inventory);
    }

    if inventory.found_none() {
        return knot_core::l10n::t("mcp.summary_none");
    }

    let rows = inventory.rows();

    // "All connected" only when that is true. Disabled, pending and unknown
    // servers are not connected, and claiming they are to save a word would
    // be the header lying about the one thing it is for.
    if rows.iter().all(|row| row.state == ServerState::Connected) {
        return knot_core::l10n::t("mcp.summary_all_connected");
    }

    knot_core::l10n::t_with("mcp.summary_no_attention",
                            &[("count", &rows.len().to_string())])
}

/// The names of the servers needing attention, capped, with a remainder
/// counting the ones the names do not cover.
fn attention_text(inventory: &Inventory) -> String {
    let needing: Vec<&ServerRow> = inventory.needing_attention().collect();
    let shown: Vec<&str> = needing.iter()
                                  .take(MAX_NAMES)
                                  .map(|row| row.name.as_str())
                                  .collect();
    let listed = shown.join(&knot_core::l10n::t("mcp.summary_separator"));
    let remainder = needing.len() - shown.len();

    if remainder == 0 {
        return knot_core::l10n::t_with("mcp.summary_attention", &[("names", &listed)]);
    }

    knot_core::l10n::t_with("mcp.summary_attention_more",
                            &[("names", &listed), ("count", &remainder.to_string())])
}

/// The label for one server state.
pub(crate) fn state_label(state: ServerState) -> String {
    let key = match state {
        ServerState::Connected => "mcp.state_connected",
        ServerState::NeedsAuthentication => "mcp.state_needs_authentication",
        ServerState::PendingApproval => "mcp.state_pending_approval",
        ServerState::Disabled => "mcp.state_disabled",
        ServerState::Failed => "mcp.state_failed",
        ServerState::Unknown => "mcp.state_unknown",
    };

    knot_core::l10n::t(key)
}

#[cfg(test)]
mod tests;
