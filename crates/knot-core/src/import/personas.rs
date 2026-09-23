//! Turning subagent definitions into personas.
//!
//! Contract: `openspec/specs/data-import/spec.md` - "Imported subagent
//! definitions become user personas".
//!
//! A definition carries no identifier of its own, so its name is the only
//! identity there is: a name already taken is treated as already imported and
//! skipped. Skipping rather than merging is deliberate - merging needs a rule
//! for whose text wins, and every such rule surprises someone.

use uuid::Uuid;

use super::result::ImportResult;
use super::subagents::SubagentDefinition;
use crate::settings::{Persona, PersonaState, PersonaType, Settings, SharedSettings};

#[cfg(test)]
mod tests;

/// Add each definition to the shared settings as a `user` persona, skipping
/// any whose name an existing persona already holds.
///
/// Personas are always `user`, never `system`: `system` personas have
/// different delete semantics and are reset by "Restore Defaults", and an
/// imported persona came from the user's own file - it must not be resettable
/// to something they never chose.
///
/// Takes the shared surface rather than a caller's `&mut Settings`, so the
/// records are added to whatever the settings hold *now*. The import window
/// used to hand over the copy it loaded when it opened, and this wrote that
/// copy back over every document - reverting any preference changed while the
/// window stood open. See `openspec/specs/settings-persistence/spec.md`,
/// "A write preserves values written elsewhere".
///
/// Only the personas document is written, and only when something was
/// actually added, so an import that skips everything leaves the store
/// untouched.
pub fn import_definitions(settings: &SharedSettings, definitions: &[SubagentDefinition])
                          -> crate::Result<ImportResult> {
    let mut result = ImportResult::default();

    // `write` may run this more than once - it retries against the newer
    // value when another writer lands first - so the result is rebuilt per
    // attempt rather than appended to across attempts.
    let installed = settings.write(|settings| {
                                result = ImportResult::default();
                                for definition in definitions {
                                    if holds_persona_named(settings, &definition.name) {
                                        result.skipped.push(definition.name.clone());
                                        continue;
                                    }
                                    settings.personas
                                            .push(Persona { id:           Uuid::new_v4(),
                                                            name:         definition.name.clone(),
                                                            instructions: definition.instructions
                                                                                    .clone(),
                                                            persona_type: PersonaType::User,
                                                            state:        PersonaState::Enabled, });
                                    result.added.push(definition.name.clone());
                                }
                            });

    if !result.added.is_empty() {
        installed.persist_personas()?;
    }
    Ok(result)
}

/// Whether a persona the user can see already goes by `name`.
///
/// Soft-deleted personas do not count. One is invisible in the persona list,
/// so reporting a definition as "already present" against a record the user
/// cannot find would be unexplainable.
fn holds_persona_named(settings: &Settings, name: &str) -> bool {
    settings.personas
            .iter()
            .filter(|persona| persona.state != PersonaState::Deleted)
            .any(|persona| persona.name == name)
}
