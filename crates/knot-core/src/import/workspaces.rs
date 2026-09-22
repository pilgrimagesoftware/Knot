//! Importing Skwad workspaces into Knot.
//!
//! Contract: `openspec/specs/data-import/spec.md` - "Importing a Skwad
//! workspace brings what it needs".
//!
//! A workspace arrives with the agents it holds, the personas those agents
//! reference, and the bench templates for their folders, so what lands is
//! usable rather than a name with broken references. Skwad's records carry
//! their own UUIDs, so identity is exact: an id Knot already holds is already
//! imported and is skipped, which is what makes re-importing safe.

use std::collections::BTreeSet;

use uuid::Uuid;

use super::result::{ImportResult, Unreadable, UnreadableReason};
use super::skwad::SkwadSource;
use crate::settings::Settings;

#[cfg(test)]
mod tests;

/// Import the workspaces in `selected` from `source` into `settings`.
///
/// Ids not present in `source` are ignored. The store is written once, and
/// only when something was actually added, so an import that skips everything
/// leaves the settings document untouched.
pub fn import_workspaces(settings: &mut Settings, source: &SkwadSource, selected: &[Uuid])
                         -> crate::Result<ImportResult> {
    let mut result = ImportResult::default();
    let selected: BTreeSet<Uuid> = selected.iter().copied().collect();

    for workspace in source.workspaces
                           .iter()
                           .filter(|w| selected.contains(&w.id))
    {
        // A workspace Knot already holds is skipped whole - its agents
        // included. Bringing the source's agents across again would add
        // records Knot's own copy of the workspace does not reference, and an
        // agent in no workspace is junk the user then has to find and delete.
        if settings.saved_workspaces
                   .iter()
                   .any(|w| w.id == workspace.id)
        {
            result.skipped.push(workspace.name.clone());
            continue;
        }
        import_agents_of(settings, source, &workspace.agent_ids, &mut result);
        settings.saved_workspaces.push(workspace.clone());
        result.added.push(workspace.name.clone());
    }

    if !result.added.is_empty() {
        settings.persist()?;
    }
    Ok(result)
}

/// Bring each of `agent_ids` across, in the order the workspace lists them,
/// along with what each agent needs to be usable.
fn import_agents_of(settings: &mut Settings, source: &SkwadSource, agent_ids: &[Uuid],
                    result: &mut ImportResult) {
    for id in agent_ids {
        let Some(agent) = source.agents.iter().find(|a| &a.id == id)
        else {
            // The workspace names an agent Skwad itself no longer holds.
            // Nothing to import and nothing to repair - the workspace keeps
            // the reference, exactly as Skwad had it.
            continue;
        };

        import_persona_of(settings, source, agent.persona_id, &agent.name, result);
        import_bench_template_for(settings, source, &agent.folder, result);

        if settings.saved_agents.iter().any(|a| &a.id == id) {
            result.skipped.push(agent.name.clone());
            continue;
        }
        settings.saved_agents.push(agent.clone());
        result.added.push(agent.name.clone());
    }
}

/// Bring the persona an agent references, if the source still holds it.
///
/// A reference the source cannot satisfy is reported and the agent goes on to
/// be imported without one, per the contract - an agent missing its persona is
/// still an agent, and dropping it would lose more than it saved.
fn import_persona_of(settings: &mut Settings, source: &SkwadSource, persona_id: Option<Uuid>,
                     agent_name: &str, result: &mut ImportResult) {
    let Some(persona_id) = persona_id
    else {
        return;
    };
    let Some(persona) = source.personas.iter().find(|p| p.id == persona_id)
    else {
        result.unreadable
              .push(Unreadable::new(agent_name, UnreadableReason::MissingPersona));
        return;
    };
    if settings.personas.iter().any(|p| p.id == persona.id) {
        result.skipped.push(persona.name.clone());
        return;
    }
    settings.personas.push(persona.clone());
    result.added.push(persona.name.clone());
}

/// Bring the bench template for `folder`, if the source has one.
///
/// Matched by folder because that is what a bench template is keyed by, and
/// skipped when Knot already has one for that folder: replacing it - which is
/// what `Settings::add_bench_agent` would do - would overwrite a template the
/// user set up, and an import never overwrites.
fn import_bench_template_for(settings: &mut Settings, source: &SkwadSource, folder: &str,
                             result: &mut ImportResult) {
    let Some(template) = source.bench_agents.iter().find(|b| b.folder == folder)
    else {
        return;
    };
    let held = settings.bench_agents
                       .iter()
                       .any(|b| b.id == template.id || b.folder == template.folder);
    if held {
        result.skipped.push(template.name.clone());
        return;
    }
    settings.bench_agents.push(template.clone());
    result.added.push(template.name.clone());
}
