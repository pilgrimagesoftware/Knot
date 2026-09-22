//! Where committed plans live: one graph per owning agent, in memory.

use std::collections::BTreeMap;

use knot_agents::AgentStore;
use knot_tasks::TaskGraph;
use uuid::Uuid;

/// The committed plans, keyed by the agent that owns each.
///
/// Runtime state, like the message queue in `mcp-messaging`: not persisted,
/// and gone when the process is. A plan whose agents are gone is not a
/// plan.
#[derive(Debug, Default)]
pub struct GraphStore {
    graphs: BTreeMap<Uuid, TaskGraph>,
}

impl GraphStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, owner: Uuid) -> Option<&TaskGraph> {
        self.graphs.get(&owner)
    }

    pub fn get_mut(&mut self, owner: Uuid) -> Option<&mut TaskGraph> {
        self.graphs.get_mut(&owner)
    }

    pub fn set(&mut self, owner: Uuid, graph: TaskGraph) {
        self.graphs.insert(owner, graph);
    }

    /// Drops plans whose owner is no longer in the agent store.
    ///
    /// Pruned on access rather than on a removal event: the tool layer sees
    /// every call but no lifecycle callback, and the observable rule -
    /// a removed agent's plan can no longer be read or dispatched from - is
    /// the same either way. Doing it here also means a re-used id cannot
    /// inherit a dead agent's plan.
    pub fn prune(&mut self, agents: &AgentStore) {
        self.graphs
            .retain(|owner, _| agents.agent(*owner).is_some());
    }

    /// How many plans are held. Test-facing: the observable behaviour is
    /// whether a given owner's plan can be read, and `get` answers that.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.graphs.len()
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.graphs.is_empty()
    }
}
