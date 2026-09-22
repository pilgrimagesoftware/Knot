//! Tests for turning subagent definitions into personas.
//!
//! Every case runs against a `Settings` pointed at a temp file, so the
//! persist-on-add path is exercised rather than stubbed out - an import that
//! adds records the app never reloads would pass a purely in-memory test.

use tempfile::TempDir;

use super::import_definitions;
use crate::import::subagents::SubagentDefinition;
use crate::settings::{Persona, PersonaState, PersonaType, Settings};

fn store(dir: &TempDir) -> Settings {
    Settings::with_store_path(dir.path().join("settings.json"))
}

fn definition(name: &str, instructions: &str) -> SubagentDefinition {
    SubagentDefinition { name:         name.into(),
                         instructions: instructions.into(),
                         source:       format!("/fixtures/{name}.md"), }
}

fn persona(name: &str, persona_type: PersonaType, state: PersonaState) -> Persona {
    Persona { id: uuid::Uuid::new_v4(),
              name: name.into(),
              instructions: "existing text".into(),
              persona_type,
              state }
}

#[test]
fn a_definition_becomes_a_persona_with_its_name_and_instructions() {
    let dir = TempDir::new().expect("temp dir");
    let mut settings = store(&dir);

    let result =
        import_definitions(&mut settings, &[definition("architect-reviewer", "You are an expert.")])
            .expect("import succeeds");

    assert_eq!(result.added, ["architect-reviewer"]);
    let imported = settings.personas
                           .iter()
                           .find(|p| p.name == "architect-reviewer")
                           .expect("added");
    assert_eq!(imported.instructions, "You are an expert.");
}

/// An imported persona must be editable and deletable like one the user wrote,
/// which is what `user` buys - and it must not be swept up by "Restore
/// Defaults", which is what `system` would have cost.
#[test]
fn an_imported_persona_is_a_user_persona() {
    let dir = TempDir::new().expect("temp dir");
    let mut settings = store(&dir);

    import_definitions(&mut settings, &[definition("one", "text")]).expect("import succeeds");

    let imported = &settings.personas[0];
    assert_eq!(imported.persona_type, PersonaType::User);
    assert_eq!(imported.state, PersonaState::Enabled);
}

/// The headline idempotency case: the same import run twice.
#[test]
fn a_second_import_of_the_same_definitions_adds_nothing() {
    let dir = TempDir::new().expect("temp dir");
    let mut settings = store(&dir);
    let definitions = [definition("one", "first"), definition("two", "second")];

    import_definitions(&mut settings, &definitions).expect("first import");
    let second = import_definitions(&mut settings, &definitions).expect("second import");

    assert!(second.added.is_empty(), "nothing new the second time");
    assert_eq!(second.skipped,
               ["one", "two"],
               "every one reported as already present");
    assert_eq!(settings.personas.len(), 2, "no duplicates");
}

/// Skipping must not quietly rewrite what is already there.
#[test]
fn a_skipped_definition_leaves_the_existing_persona_alone() {
    let dir = TempDir::new().expect("temp dir");
    let mut settings = store(&dir);
    settings.personas.push(persona("architect-reviewer",
                                   PersonaType::User,
                                   PersonaState::Enabled));

    import_definitions(&mut settings,
                       &[definition("architect-reviewer", "new text")]).expect("import succeeds");

    assert_eq!(settings.personas.len(), 1);
    assert_eq!(settings.personas[0].instructions, "existing text");
}

#[test]
fn a_name_taken_by_a_system_persona_is_also_skipped() {
    let dir = TempDir::new().expect("temp dir");
    let mut settings = store(&dir);
    settings.personas
            .push(persona("Kent Beck", PersonaType::System, PersonaState::Enabled));

    let result = import_definitions(&mut settings, &[definition("Kent Beck", "text")])
        .expect("import succeeds");

    assert_eq!(result.skipped, ["Kent Beck"]);
    assert_eq!(settings.personas.len(), 1);
}

/// A soft-deleted persona is invisible in the persona list, so reporting a
/// definition as "already present" against it would be unexplainable.
#[test]
fn a_name_held_only_by_a_deleted_persona_is_free_to_import() {
    let dir = TempDir::new().expect("temp dir");
    let mut settings = store(&dir);
    settings.personas
            .push(persona("Kent Beck", PersonaType::System, PersonaState::Deleted));

    let result = import_definitions(&mut settings, &[definition("Kent Beck", "fresh text")])
        .expect("import succeeds");

    assert_eq!(result.added, ["Kent Beck"]);
}

/// `data-import`: an import never removes or reorders what the user has.
#[test]
fn existing_personas_keep_their_contents_and_order() {
    let dir = TempDir::new().expect("temp dir");
    let mut settings = store(&dir);
    settings.personas
            .push(persona("first", PersonaType::User, PersonaState::Enabled));
    settings.personas
            .push(persona("second", PersonaType::System, PersonaState::Enabled));
    let before = settings.personas.clone();

    import_definitions(&mut settings, &[definition("third", "text")]).expect("import succeeds");

    assert_eq!(&settings.personas[..2],
               &before[..],
               "existing personas untouched and in order");
    assert_eq!(settings.personas[2].name, "third",
               "the new one is appended");
}

#[test]
fn an_import_that_adds_nothing_is_still_a_success() {
    let dir = TempDir::new().expect("temp dir");
    let mut settings = store(&dir);

    let result = import_definitions(&mut settings, &[]).expect("import succeeds");

    assert!(result.is_empty());
    assert!(settings.personas.is_empty());
}

/// Added personas have to survive the round trip, or the app reloads without
/// them.
#[test]
fn imported_personas_are_persisted() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("settings.json");
    let mut settings = Settings::with_store_path(&path);

    import_definitions(&mut settings, &[definition("one", "text")]).expect("import succeeds");

    let reloaded = Settings::load_from(&path).expect("reload");
    assert_eq!(reloaded.personas.len(), 1);
    assert_eq!(reloaded.personas[0].name, "one");
}
