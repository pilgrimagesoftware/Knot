use std::collections::BTreeMap;
use std::path::PathBuf;

use uuid::Uuid;

use super::AgentStore;
use crate::error::{AgentError, Result};

impl AgentStore {
    pub fn set_markdown_panel(&mut self, id: Uuid, file: PathBuf, maximized: bool) -> Result<()> {
        let agent = self.agent_mut(id).ok_or(AgentError::NotFound(id))?;
        agent.markdown_history.retain(|path| *path != file);
        agent.markdown_history.insert(0, file.clone());
        agent.markdown_file = Some(file);
        agent.markdown_maximized = maximized;
        Ok(())
    }

    /// Closes the markdown panel without touching the history, so the file
    /// stays reachable from that agent's "Markdown Files" menu afterwards.
    pub fn clear_markdown_panel(&mut self, id: Uuid) -> Result<()> {
        let agent = self.agent_mut(id).ok_or(AgentError::NotFound(id))?;
        agent.markdown_file = None;
        agent.markdown_maximized = false;
        Ok(())
    }

    pub fn set_mermaid_panel(&mut self, id: Uuid, source: String, title: Option<String>)
                             -> Result<()> {
        let agent = self.agent_mut(id).ok_or(AgentError::NotFound(id))?;
        agent.mermaid_source = Some(source);
        agent.mermaid_title = title;
        Ok(())
    }

    /// Records one Panel session-setup selection (model, permission mode,
    /// reasoning effort, ...) so reopening the agent can replay it.
    ///
    /// `config_id` is the id the adapter declared for the option, not a
    /// vocabulary Knot defines. Memory only - the caller persists.
    pub fn set_session_config_option(&mut self, id: Uuid, config_id: String, value: String)
                                     -> Result<()> {
        let agent = self.agent_mut(id).ok_or(AgentError::NotFound(id))?;
        agent.session_config.insert(config_id, value);
        Ok(())
    }

    /// The setup to replay when this agent's session is (re)created. Empty
    /// for an agent saved before the field existed, which leaves the
    /// adapter's own defaults in place.
    pub fn session_config(&self, id: Uuid) -> BTreeMap<String, String> {
        self.agent(id)
            .map(|agent| agent.session_config.clone())
            .unwrap_or_default()
    }
}
