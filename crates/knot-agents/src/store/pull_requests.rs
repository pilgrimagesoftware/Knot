//! The pull requests Knot has seen in its agents' output.
//!
//! Contract: the `pull-request-tracking` capability spec under
//! `openspec/changes/pull-request-tracking/specs/`.
//!
//! The collection lives here rather than beside the settings document because
//! this is where an agent stops existing. Cascading a removal from the store's
//! own removal paths is what keeps a record from outliving the agent it is
//! attributed to - a record with no agent has nothing to show it under.

use knot_core::SavedPullRequest;
use uuid::Uuid;

use super::AgentStore;

impl AgentStore {
    /// Every recorded pull request, in the order they were first seen.
    pub fn pull_requests(&self) -> &[SavedPullRequest] {
        &self.pull_requests
    }

    /// Replace the collection wholesale, as the settings document supplies it
    /// at load.
    pub fn set_pull_requests(&mut self, records: Vec<SavedPullRequest>) {
        self.pull_requests = records;
    }

    /// Record `url` against `agent_id`, returning whether this was new.
    ///
    /// Idempotent per agent: seeing the same URL again - because it scrolled
    /// past twice, or because a tool call was re-rendered - leaves the
    /// existing record exactly as it is, first-seen time included. A caller
    /// can therefore offer every URL it sees without checking first, which is
    /// what both detection taps do.
    ///
    /// The same pull request from a second agent *is* new, because who opened
    /// it is part of what is recorded.
    ///
    /// An unknown `agent_id` records nothing: an agent can be removed while
    /// output about it is still in flight, and a record with no agent has
    /// nothing to show it under.
    pub fn record_pull_request(&mut self, agent_id: Uuid, url: impl Into<String>) -> bool {
        let url = url.into();
        if self.pull_requests
               .iter()
               .any(|record| record.is_same_sighting(&url, agent_id))
        {
            return false;
        }
        let Some(workspace_id) = self.workspace_of(agent_id)
        else {
            return false;
        };
        self.pull_requests
            .push(SavedPullRequest::new(url, agent_id, workspace_id));
        true
    }

    /// Forget one recorded pull request, returning whether there was one.
    ///
    /// Knot's record only: nothing is closed, deleted or changed on the
    /// forge. Because Knot records what it sees, the same URL appearing in
    /// the agent's output again records it again.
    pub fn remove_pull_request(&mut self, agent_id: Uuid, url: &str) -> bool {
        let before = self.pull_requests.len();
        self.pull_requests
            .retain(|record| !record.is_same_sighting(url, agent_id));
        self.pull_requests.len() != before
    }

    /// One workspace's recorded pull requests, newest first.
    pub fn pull_requests_for_workspace(&self, workspace_id: Uuid) -> Vec<&SavedPullRequest> {
        self.sorted_newest_first(|record| record.workspace_id == workspace_id)
    }

    /// One agent's recorded pull requests, newest first.
    pub fn pull_requests_for_agent(&self, agent_id: Uuid) -> Vec<&SavedPullRequest> {
        self.sorted_newest_first(|record| record.agent_id == agent_id)
    }

    /// Ties break on the URL rather than on position, so a workspace whose
    /// records all arrived within one second still lists in a stable order
    /// rather than shuffling between frames.
    fn sorted_newest_first(&self, keep: impl Fn(&SavedPullRequest) -> bool)
                           -> Vec<&SavedPullRequest> {
        let mut records = self.pull_requests
                              .iter()
                              .filter(|record| keep(record))
                              .collect::<Vec<_>>();
        records.sort_by(|left, right| {
                   right.first_seen
                        .cmp(&left.first_seen)
                        .then_with(|| left.url.cmp(&right.url))
               });
        records
    }

    /// Drop every record attributed to `agent_id`. Called from the store's
    /// agent-removal path, so there is one place that knows an agent is going
    /// away.
    pub(super) fn forget_agent_pull_requests(&mut self, agent_id: Uuid) {
        self.pull_requests
            .retain(|record| record.agent_id != agent_id);
    }

    /// Drop every record attributed to `workspace_id`.
    ///
    /// Removing a workspace already removes its agents, which takes their
    /// records with them. This catches the one case that does not: a record
    /// seen while its agent was in this workspace, whose agent has since moved
    /// to another one. The record names the workspace it was seen in, so it
    /// would otherwise point at a workspace that no longer exists.
    pub(super) fn forget_workspace_pull_requests(&mut self, workspace_id: Uuid) {
        self.pull_requests
            .retain(|record| record.workspace_id != workspace_id);
    }
}
