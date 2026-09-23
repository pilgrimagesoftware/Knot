//! Tests for turning subagent definitions into personas.
//!
//! Every case runs against a settings surface pointed at a temp file, so the
//! persist-on-add path is exercised rather than stubbed out - an import that
//! adds records the app never reloads would pass a purely in-memory test.
//!
//! The import takes the shared surface rather than a `&mut Settings`, so a
//! case that seeds existing personas builds the value first and shares it.
//! Assertions read back through `shared.read()`: that is the value the import
//! installed, and reading a local the import never saw would assert against
//! the wrong thing.

use tempfile::TempDir;

use super::import_definitions;
use crate::import::subagents::SubagentDefinition;
use crate::settings::{Persona, PersonaState, PersonaType, Settings, SharedSettings};

fn store(dir: &TempDir) -> Settings {
    Settings::with_store_root(dir.path())
}

fn shared(dir: &TempDir) -> SharedSettings {
    SharedSettings::new(store(dir))
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
    let shared = shared(&dir);

    let result =
        import_definitions(&shared, &[definition("architect-reviewer", "You are an expert.")])
            .expect("import succeeds");

    assert_eq!(result.added, ["architect-reviewer"]);
    let settings = shared.read();
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
    let shared = shared(&dir);

    import_definitions(&shared, &[definition("one", "text")]).expect("import succeeds");

    let settings = shared.read();
    let imported = &settings.personas[0];
    assert_eq!(imported.persona_type, PersonaType::User);
    assert_eq!(imported.state, PersonaState::Enabled);
}

/// The headline idempotency case: the same import run twice.
#[test]
fn a_second_import_of_the_same_definitions_adds_nothing() {
    let dir = TempDir::new().expect("temp dir");
    let shared = shared(&dir);
    let definitions = [definition("one", "first"), definition("two", "second")];

    import_definitions(&shared, &definitions).expect("first import");
    let second = import_definitions(&shared, &definitions).expect("second import");

    assert!(second.added.is_empty(), "nothing new the second time");
    assert_eq!(second.skipped,
               ["one", "two"],
               "every one reported as already present");
    assert_eq!(shared.read().personas.len(), 2, "no duplicates");
}

/// Skipping must not quietly rewrite what is already there.
#[test]
fn a_skipped_definition_leaves_the_existing_persona_alone() {
    let dir = TempDir::new().expect("temp dir");
    let mut settings = store(&dir);
    settings.personas.push(persona("architect-reviewer",
                                   PersonaType::User,
                                   PersonaState::Enabled));
    let shared = SharedSettings::new(settings);

    import_definitions(&shared,
                       &[definition("architect-reviewer", "new text")]).expect("import succeeds");

    let settings = shared.read();
    assert_eq!(settings.personas.len(), 1);
    assert_eq!(settings.personas[0].instructions, "existing text");
}

#[test]
fn a_name_taken_by_a_system_persona_is_also_skipped() {
    let dir = TempDir::new().expect("temp dir");
    let mut settings = store(&dir);
    settings.personas
            .push(persona("Kent Beck", PersonaType::System, PersonaState::Enabled));
    let shared = SharedSettings::new(settings);

    let result =
        import_definitions(&shared, &[definition("Kent Beck", "text")]).expect("import succeeds");

    assert_eq!(result.skipped, ["Kent Beck"]);
    assert_eq!(shared.read().personas.len(), 1);
}

/// A soft-deleted persona is invisible in the persona list, so reporting a
/// definition as "already present" against it would be unexplainable.
#[test]
fn a_name_held_only_by_a_deleted_persona_is_free_to_import() {
    let dir = TempDir::new().expect("temp dir");
    let mut settings = store(&dir);
    settings.personas
            .push(persona("Kent Beck", PersonaType::System, PersonaState::Deleted));
    let shared = SharedSettings::new(settings);

    let result = import_definitions(&shared, &[definition("Kent Beck", "fresh text")])
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
    let shared = SharedSettings::new(settings);

    import_definitions(&shared, &[definition("third", "text")]).expect("import succeeds");

    let settings = shared.read();
    assert_eq!(&settings.personas[..2],
               &before[..],
               "existing personas untouched and in order");
    assert_eq!(settings.personas[2].name, "third",
               "the new one is appended");
}

#[test]
fn an_import_that_adds_nothing_is_still_a_success() {
    let dir = TempDir::new().expect("temp dir");
    let shared = shared(&dir);

    let result = import_definitions(&shared, &[]).expect("import succeeds");

    assert!(result.is_empty());
    assert!(shared.read().personas.is_empty());
}

/// Added personas have to survive the round trip, or the app reloads without
/// them.
#[test]
fn imported_personas_are_persisted() {
    let dir = TempDir::new().expect("temp dir");
    let shared = shared(&dir);

    import_definitions(&shared, &[definition("one", "text")]).expect("import succeeds");

    let reloaded = Settings::load_from_root(dir.path()).expect("reload");
    assert_eq!(reloaded.personas.len(), 1);
    assert_eq!(reloaded.personas[0].name, "one");
}

/// The write half of #238, at one of the two sites that still had it. The
/// import window handed over the copy it loaded when it opened, and the
/// import wrote that copy back over every document - so a preference changed
/// while the window stood open was reverted by the next import.
///
/// `openspec/specs/settings-persistence/spec.md`, "A write preserves values
/// written elsewhere" - scenario "An import does not revert a preference".
#[test]
fn an_import_does_not_revert_a_preference_changed_while_it_was_open() {
    let dir = TempDir::new().expect("temp dir");
    let shared = shared(&dir);
    // The import window opens and reads the surface, as it does today.
    let held_since_the_window_opened = shared.read();

    // The settings window changes a preference and writes it.
    shared.write(|settings| settings.mcp_server_port = 9000);
    shared.read()
          .persist_preferences()
          .expect("the settings window writes preferences");

    // Only now does the user run the import.
    import_definitions(&shared, &[definition("one", "text")]).expect("import succeeds");

    assert_ne!(held_since_the_window_opened.mcp_server_port, 9000,
               "the window's early read must genuinely be stale, or this proves nothing");
    assert_eq!(shared.read().mcp_server_port,
               9000,
               "the import must not revert the preference");
    let reloaded = Settings::load_from_root(dir.path()).expect("reload");
    assert_eq!(reloaded.mcp_server_port, 9000,
               "and must not revert it on disk either");
    assert_eq!(reloaded.personas.len(), 1, "while still importing");
}
