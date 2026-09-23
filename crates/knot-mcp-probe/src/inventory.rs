//! What a probe found.
//!
//! The whole point of this type is that "I cannot tell you" and "there are
//! none" are different answers and must not share a representation. An empty
//! `Vec` would say both, and the one the section would show is the wrong one:
//! telling a user their agent has no MCP servers when the truth is that Knot
//! has no way to ask is worse than saying nothing.
//!
//! So [`Inventory::Unprobeable`] is a variant, not an empty list, and it is
//! an `Ok` value rather than a [`crate::ProbeError`] - an agent type Knot
//! cannot interrogate is a supported configuration, the way a machine without
//! `gh` is for `knot-forge`.

use std::time::Instant;

use crate::server::ServerRow;
use crate::state::ServerState;

/// The outcome of asking an agent which MCP servers it has.
#[derive(Debug, Clone)]
pub enum Inventory {
    /// This agent type has no MCP list command, so nothing was asked.
    Unprobeable,
    /// A probe ran and these are its rows. An empty `rows` means the agent
    /// genuinely has no MCP servers configured.
    Probed { rows: Vec<ServerRow>, at: Instant },
}

impl Inventory {
    /// A completed probe, stamped now.
    #[must_use]
    pub fn probed(rows: Vec<ServerRow>) -> Self {
        Self::Probed { rows,
                       at: Instant::now() }
    }

    /// The rows found. Empty for [`Self::Unprobeable`] - callers that care
    /// about the difference must ask [`Self::is_unprobeable`], which is why
    /// that is a method and not a matter of reading the length.
    #[must_use]
    pub fn rows(&self) -> &[ServerRow] {
        match self {
            Self::Unprobeable => &[],
            Self::Probed { rows, .. } => rows,
        }
    }

    /// When the probe ran, for the age the section shows beside its rows.
    #[must_use]
    pub fn taken_at(&self) -> Option<Instant> {
        match self {
            Self::Unprobeable => None,
            Self::Probed { at, .. } => Some(*at),
        }
    }

    #[must_use]
    pub fn is_unprobeable(&self) -> bool {
        matches!(self, Self::Unprobeable)
    }

    /// Whether a probe ran and found nothing configured.
    ///
    /// Deliberately false for [`Self::Unprobeable`]: that is the case this
    /// type exists to keep separate.
    #[must_use]
    pub fn found_none(&self) -> bool {
        matches!(self, Self::Probed { rows, .. } if rows.is_empty())
    }

    /// The rows the user is being asked to do something about.
    pub fn needing_attention(&self) -> impl Iterator<Item = &ServerRow> {
        self.rows().iter().filter(|row| row.state.needs_attention())
    }

    /// How many rows need attention.
    #[must_use]
    pub fn attention_count(&self) -> usize {
        self.needing_attention().count()
    }

    /// Whether any row is in a given state.
    #[must_use]
    pub fn any_in(&self, state: ServerState) -> bool {
        self.rows().iter().any(|row| row.state == state)
    }
}

#[cfg(test)]
mod tests;
