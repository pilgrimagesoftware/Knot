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
use crate::settings::{Settings, SharedSettings};

#[cfg(test)]
mod tests;

/// Which collection documents an import added to, and so which to write.
///
/// An import reaches four collections and rarely changes all of them. Writing
/// only what changed is the spec's "A write touches only the document it
/// belongs to"; writing the whole surface is what reverted preferences, and
/// writing every collection unconditionally would still rewrite documents
/// this import never touched.
#[derive(Default)]
struct Touched {
    workspaces: bool,
    agents:     bool,
    personas:   bool,
    bench:      bool,
}

impl Touched {
    /// Write each document this import added to, and no others.
    fn persist(&self, settings: &Settings) -> crate::Result<()> {
        if self.agents {
            settings.persist_agents()?;
        }
        if self.workspaces {
            settings.persist_workspaces()?;
        }
        if self.personas {
            settings.persist_personas()?;
        }
        if self.bench {
            settings.persist_bench()?;
        }
        Ok(())
    }
}

/// Import the workspaces in `selected` from `source` into the shared
/// settings.
///
/// Takes the shared surface rather than a caller's `&mut Settings`, so the
/// records land on whatever the settings hold *now*. The import window used
/// to hand over the copy it loaded when it opened, and this wrote that copy
/// back over every document - reverting any preference changed while the
/// window stood open. See `openspec/specs/settings-persistence/spec.md`,
/// "A write preserves values written elsewhere".
///
/// Ids not present in `source` are ignored. Only the collection documents
/// this import actually added to are written, and only when something was
/// added at all, so an import that skips everything leaves the store
/// untouched.
pub fn import_workspaces(settings: &SharedSettings, source: &SkwadSource, selected: &[Uuid])
                         -> crate::Result<ImportResult> {
    let mut result = ImportResult::default();
    let mut touched = Touched::default();
    let selected: BTreeSet<Uuid> = selected.iter().copied().collect();

    // `write` may run this more than once - it retries against the newer
    // value when another writer lands first - so both the result and the
    // touched set are rebuilt per attempt rather than accumulated across
    // attempts.
    let installed =
        settings.write(|settings| {
                    result = ImportResult::default();
                    touched = Touched::default();
                    for workspace in source.workspaces
                                           .iter()
                                           .filter(|w| selected.contains(&w.id))
                    {
                        // A workspace Knot already holds is skipped whole - its
                        // agents included. Bringing the source's agents across
                        // again would add records Knot's own copy of the
                        // workspace does not reference,
                        // and an agent in no workspace is junk
                        // the user then has to find and delete.
                        if settings.saved_workspaces
                                   .iter()
                                   .any(|w| w.id == workspace.id)
                        {
                            result.skipped.push(workspace.name.clone());
                            continue;
                        }
                        import_agents_of(settings,
                                         source,
                                         &workspace.agent_ids,
                                         &mut result,
                                         &mut touched);
                        settings.saved_workspaces.push(workspace.clone());
                        touched.workspaces = true;
                        result.added.push(workspace.name.clone());
                    }
                });

    if !result.added.is_empty() {
        touched.persist(&installed)?;
    }
    Ok(result)
}

/// Bring each of `agent_ids` across, in the order the workspace lists them,
/// along with what each agent needs to be usable.
fn import_agents_of(settings: &mut Settings, source: &SkwadSource, agent_ids: &[Uuid],
                    result: &mut ImportResult, touched: &mut Touched) {
    for id in agent_ids {
        let Some(agent) = source.agents.iter().find(|a| &a.id == id)
        else {
            // The workspace names an agent Skwad itself no longer holds.
            // Nothing to import and nothing to repair - the workspace keeps
            // the reference, exactly as Skwad had it.
            continue;
        };

        import_persona_of(settings,
                          source,
                          agent.persona_id,
                          &agent.name,
                          result,
                          touched);
        import_bench_template_for(settings, source, &agent.folder, result, touched);

        if settings.saved_agents.iter().any(|a| &a.id == id) {
            result.skipped.push(agent.name.clone());
            continue;
        }
        settings.saved_agents.push(agent.clone());
        touched.agents = true;
        result.added.push(agent.name.clone());
    }
}

/// Bring the persona an agent references, if the source still holds it.
///
/// A reference the source cannot satisfy is reported and the agent goes on to
/// be imported without one, per the contract - an agent missing its persona is
/// still an agent, and dropping it would lose more than it saved.
fn import_persona_of(settings: &mut Settings, source: &SkwadSource, persona_id: Option<Uuid>,
                     agent_name: &str, result: &mut ImportResult, touched: &mut Touched) {
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
    touched.personas = true;
    result.added.push(persona.name.clone());
}

/// Bring the bench template for `folder`, if the source has one.
///
/// Matched by folder because that is what a bench template is keyed by, and
/// skipped when Knot already has one for that folder: replacing it - which is
/// what `Settings::add_bench_agent` would do - would overwrite a template the
/// user set up, and an import never overwrites.
fn import_bench_template_for(settings: &mut Settings, source: &SkwadSource, folder: &str,
                             result: &mut ImportResult, touched: &mut Touched) {
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
    touched.bench = true;
    result.added.push(template.name.clone());
}
