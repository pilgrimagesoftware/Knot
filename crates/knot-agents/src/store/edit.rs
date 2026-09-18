use uuid::Uuid;

use super::{AgentStore, EditRequest};
use crate::error::{AgentError, Result};

impl AgentStore {
    pub fn edit(&mut self, id: Uuid, req: EditRequest) -> Result<()> {
        let old_folder = self.agent(id)
                             .ok_or(AgentError::NotFound(id))?
                             .folder
                             .clone();
        let mut needs_restart = false;
        {
            let agent = self.agent_mut(id).expect("agent checked above");
            agent.name = req.name;
            agent.avatar = req.avatar;
            if let Some(agent_type) = &req.agent_type
               && *agent_type != agent.agent_type
            {
                agent.agent_type = agent_type.clone();
                needs_restart = true;
            }
            if req.persona_changed {
                agent.persona_id = req.persona_id;
                needs_restart = true;
            }
            if let Some(folder) = &req.folder
               && *folder != old_folder
            {
                agent.folder = folder.clone();
                needs_restart = true;
            }
        }
        if let Some(new_folder) = req.folder.as_ref().filter(|folder| **folder != old_folder)
           && req.relocate_companions
        {
            let stale_companions = self.companions(id)
                                       .into_iter()
                                       .filter(|companion| {
                                           self.agent(*companion)
                                               .is_some_and(|agent| agent.folder == old_folder)
                                       })
                                       .collect::<Vec<_>>();
            for companion_id in stale_companions {
                if let Some(agent) = self.agent_mut(companion_id) {
                    agent.folder = new_folder.clone();
                }
                self.restart(companion_id)?;
            }
        }
        if needs_restart {
            self.restart(id)?;
        }
        Ok(())
    }
}
