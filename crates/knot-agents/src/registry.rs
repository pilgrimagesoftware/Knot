//! The agent registry: a read-only projection over the live agents and the
//! bench templates, answering "who can do X" by capability tag.
//!
//! Contract: `openspec/specs/agent-registry/spec.md`.
//!
//! Deliberately *not* a stored collection. Every field it reports already
//! lives on a `SavedAgent` or a `BenchAgent`; a second copy keyed by agent
//! id would diverge the first time an agent was edited outside the registry
//! path. The projection is computed on demand instead.
//!
//! It is also ignorant of what an agent type can actually run. The tool
//! surface an ACP adapter declares is known to the MCP layer, not here, so
//! [`RegistryView`] takes a resolver for it. Keeping that out means the
//! matching and ranking rules can be tested without an adapter, a server,
//! or a runtime.

use knot_core::{BenchAgent, Capabilities, CostTier};
use uuid::Uuid;

use crate::agent::{Agent, AgentState};
use crate::store::AgentStore;

/// What a registry entry is doing right now.
///
/// A template has no session, so it reports no state at all rather than a
/// stand-in one - "idle" would read as an agent sitting ready.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryStatus {
    Live {
        state:         AgentState,
        status_text:   String,
        is_registered: bool,
    },
    Template,
}

impl RegistryStatus {
    /// Whether this entry is a live agent with nothing in flight - the
    /// first rank group, since it can start now.
    pub fn is_idle_agent(&self) -> bool {
        matches!(self,
                 Self::Live { state: AgentState::Idle,
                              .. })
    }

    fn rank(&self) -> u8 {
        match self {
            Self::Live { state: AgentState::Idle,
                         .. } => 0,
            Self::Live { .. } => 1,
            Self::Template => 2,
        }
    }
}

/// One candidate: everything a caller needs to choose it, so a query answer
/// never needs a second call to interpret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryEntry {
    pub id:           Uuid,
    pub name:         String,
    pub folder:       String,
    pub description:  String,
    pub capabilities: Capabilities,
    /// The agent type, followed by whatever tool surface its adapter
    /// declares. Always carries at least the type.
    pub tools:        Vec<String>,
    pub cost_tier:    CostTier,
    pub status:       RegistryStatus,
}

impl RegistryEntry {
    /// Sort key: soonest-available first, then cheapest, then by name.
    /// Ordering is a recommendation, not an assignment - the caller still
    /// chooses - but a caller that takes the first entry should get the
    /// cheapest agent able to start now.
    fn sort_key(&self) -> (u8, CostTier, String) {
        (self.status.rank(), self.cost_tier, self.name.to_lowercase())
    }
}

/// What a caller is looking for. An empty tag set matches everything, which
/// is how "list them all" works.
#[derive(Debug, Clone, Default)]
pub struct RegistryQuery {
    pub capabilities:      Capabilities,
    pub include_templates: bool,
}

impl RegistryQuery {
    /// Every visible entry, live agents and templates alike.
    pub fn everything() -> Self {
        Self { capabilities:      Capabilities::new(),
               include_templates: true, }
    }

    pub fn for_capabilities(capabilities: Capabilities) -> Self {
        Self { capabilities,
               include_templates: true }
    }
}

/// A registry reading over one store and one bench.
///
/// `tools_for` maps an agent type to the tool surface its adapter declares;
/// the type itself is always included, so a resolver returning nothing still
/// produces a usable entry.
pub struct RegistryView<'a> {
    store:     &'a AgentStore,
    bench:     &'a [BenchAgent],
    tools_for: &'a dyn Fn(&str) -> Vec<String>,
}

impl<'a> RegistryView<'a> {
    pub fn new(store: &'a AgentStore, bench: &'a [BenchAgent],
               tools_for: &'a dyn Fn(&str) -> Vec<String>)
               -> Self {
        Self { store,
               bench,
               tools_for }
    }

    /// The candidates `caller_id` may see that carry every requested tag,
    /// ranked. Visibility follows the same rule as listing agents: the
    /// caller's own workspace, and companions only where the caller owns
    /// them.
    ///
    /// A query matching nothing yields an empty list. That is an answer an
    /// orchestrator can act on - "there is no reviewer" - not a failure.
    pub fn candidates(&self, caller_id: Uuid, query: &RegistryQuery) -> Vec<RegistryEntry> {
        let mut entries: Vec<RegistryEntry> =
            self.visible_agents(caller_id)
                .into_iter()
                .filter(|agent| agent.capabilities.contains_all(&query.capabilities))
                .map(|agent| self.entry_for_agent(agent))
                .collect();
        if query.include_templates {
            entries.extend(self.bench
                               .iter()
                               .filter(|bench| bench.capabilities.contains_all(&query.capabilities))
                               .map(|bench| self.entry_for_bench(bench)));
        }
        entries.sort_by_key(RegistryEntry::sort_key);
        entries
    }

    /// Every agent in `caller_id`'s workspace, minus companions it does not
    /// own. Mirrors the visibility rule `list-agents` already applies.
    fn visible_agents(&self, caller_id: Uuid) -> Vec<&'a Agent> {
        self.store
            .workspaces()
            .iter()
            .find(|workspace| workspace.agent_ids.contains(&caller_id))
            .map(|workspace| {
                workspace.agent_ids
                         .iter()
                         .filter_map(|id| self.store.agent(*id))
                         .filter(|agent| !agent.is_companion || agent.created_by == Some(caller_id))
                         .collect()
            })
            .unwrap_or_default()
    }

    fn tools(&self, agent_type: &str) -> Vec<String> {
        let mut tools = vec![agent_type.to_string()];
        tools.extend((self.tools_for)(agent_type));
        tools
    }

    fn entry_for_agent(&self, agent: &Agent) -> RegistryEntry {
        RegistryEntry { id:           agent.id,
                        name:         agent.name.clone(),
                        folder:       agent.folder.clone(),
                        description:  agent.description.clone(),
                        capabilities: agent.capabilities.clone(),
                        tools:        self.tools(&agent.agent_type),
                        cost_tier:    agent.cost_tier,
                        status:       RegistryStatus::Live { state:         agent.state,
                                                             status_text:   agent.status_text
                                                                                 .clone(),
                                                             is_registered: agent.is_registered, }, }
    }

    fn entry_for_bench(&self, bench: &BenchAgent) -> RegistryEntry {
        RegistryEntry { id:           bench.id,
                        name:         bench.name.clone(),
                        folder:       bench.folder.clone(),
                        description:  bench.description.clone(),
                        capabilities: bench.capabilities.clone(),
                        tools:        self.tools(&bench.agent_type),
                        cost_tier:    bench.cost_tier,
                        status:       RegistryStatus::Template, }
    }
}

#[cfg(test)]
mod tests;
