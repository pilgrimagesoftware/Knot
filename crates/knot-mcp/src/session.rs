use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use uuid::Uuid;

use crate::consts;

/// A tracked MCP session for one agent.
#[derive(Debug, Clone)]
pub struct McpSession {
    pub id:            String,
    pub agent_id:      Uuid,
    pub created_at:    Instant,
    pub last_activity: Instant,
}

impl McpSession {
    fn new(agent_id: Uuid) -> Self {
        let now = Instant::now();
        Self { id: Uuid::new_v4().to_string(),
               agent_id,
               created_at: now,
               last_activity: now }
    }
}

#[derive(Default)]
struct SessionTable {
    sessions:         HashMap<String, McpSession>,
    agent_to_session: HashMap<Uuid, String>,
}

/// One session per agent, keyed by an opaque id. Creating a session for an
/// agent that already has one replaces the old session.
#[derive(Clone, Default)]
pub struct McpSessionManager {
    table: Arc<Mutex<SessionTable>>,
}

impl McpSessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a session for `agent_id`. `Uuid::nil()` marks a
    /// pre-registration session (the agent isn't identified yet, e.g. during
    /// the `initialize` handshake) and is exempt from the one-session-per-agent
    /// dedup below — otherwise every new agent's handshake would evict every
    /// other still-registering agent's session, since they'd all share the
    /// nil key.
    pub fn create_session(&self, agent_id: Uuid) -> McpSession {
        let mut table = self.table.lock();
        let session = McpSession::new(agent_id);
        if agent_id != Uuid::nil() {
            if let Some(old_id) = table.agent_to_session.remove(&agent_id) {
                table.sessions.remove(&old_id);
            }
            table.agent_to_session.insert(agent_id, session.id.clone());
        }
        table.sessions.insert(session.id.clone(), session.clone());
        session
    }

    pub fn session(&self, id: &str) -> Option<McpSession> {
        self.table.lock().sessions.get(id).cloned()
    }

    /// How many sessions are live right now - what the heartbeat reports, so
    /// an idle server can be told from one nothing is connected to.
    pub fn len(&self) -> usize {
        self.table.lock().sessions.len()
    }

    /// Whether no session is live. Paired with [`Self::len`] because clippy
    /// asks for it; the heartbeat uses the count.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn session_for_agent(&self, agent_id: Uuid) -> Option<McpSession> {
        let table = self.table.lock();
        let id = table.agent_to_session.get(&agent_id)?;
        table.sessions.get(id).cloned()
    }

    pub fn touch(&self, id: &str) {
        if let Some(session) = self.table.lock().sessions.get_mut(id) {
            session.last_activity = Instant::now();
        }
    }

    pub fn remove(&self, id: &str) {
        let mut table = self.table.lock();
        if let Some(session) = table.sessions.remove(id) {
            table.agent_to_session.remove(&session.agent_id);
        }
    }

    pub fn remove_for_agent(&self, agent_id: Uuid) {
        let mut table = self.table.lock();
        if let Some(id) = table.agent_to_session.remove(&agent_id) {
            table.sessions.remove(&id);
        }
    }

    /// Removes sessions idle longer than the crate's default timeout (1 hour).
    pub fn cleanup_stale_default(&self) {
        self.cleanup_stale(consts::DEFAULT_SESSION_TIMEOUT);
    }

    /// Removes sessions whose `last_activity` is older than `timeout`.
    pub fn cleanup_stale(&self, timeout: Duration) {
        let mut table = self.table.lock();
        let stale: Vec<String> = table.sessions
                                      .values()
                                      .filter(|s| s.last_activity.elapsed() > timeout)
                                      .map(|s| s.id.clone())
                                      .collect();
        for id in stale {
            if let Some(session) = table.sessions.remove(&id) {
                table.agent_to_session.remove(&session.agent_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_session_per_agent() {
        let manager = McpSessionManager::new();
        let agent_id = Uuid::new_v4();

        let first = manager.create_session(agent_id);
        let second = manager.create_session(agent_id);

        assert!(manager.session(&first.id).is_none());
        assert!(manager.session(&second.id).is_some());
        assert_eq!(manager.session_for_agent(agent_id).unwrap().id, second.id);
    }

    #[test]
    fn nil_agent_sessions_do_not_evict_each_other() {
        let manager = McpSessionManager::new();

        let first = manager.create_session(Uuid::nil());
        let second = manager.create_session(Uuid::nil());

        assert!(manager.session(&first.id).is_some());
        assert!(manager.session(&second.id).is_some());
    }

    #[test]
    fn the_live_count_tracks_creation_replacement_and_cleanup() {
        let manager = McpSessionManager::new();
        assert_eq!(manager.len(), 0);
        assert!(manager.is_empty());

        let agent = Uuid::new_v4();
        manager.create_session(agent);
        manager.create_session(Uuid::new_v4());
        assert_eq!(manager.len(), 2);
        assert!(!manager.is_empty());

        // Replacing an agent's session leaves the count alone: the old one
        // is removed as the new one lands, so a reconnecting agent must not
        // read as a second connection.
        manager.create_session(agent);
        assert_eq!(manager.len(), 2);

        manager.remove_for_agent(agent);
        assert_eq!(manager.len(), 1);

        std::thread::sleep(Duration::from_millis(20));
        manager.cleanup_stale(Duration::from_millis(10));
        assert_eq!(manager.len(), 0, "a reclaimed session stops being counted");
    }

    #[test]
    fn cleanup_stale_removes_only_expired_sessions() {
        let manager = McpSessionManager::new();
        let stale_agent = Uuid::new_v4();
        let fresh_agent = Uuid::new_v4();

        let stale = manager.create_session(stale_agent);
        std::thread::sleep(Duration::from_millis(30));
        let fresh = manager.create_session(fresh_agent);

        manager.cleanup_stale(Duration::from_millis(15));

        assert!(manager.session(&stale.id).is_none());
        assert!(manager.session(&fresh.id).is_some());
    }
}
