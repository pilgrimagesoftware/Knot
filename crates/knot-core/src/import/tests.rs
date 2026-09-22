//! End-to-end checks across the whole import surface, covering the change's
//! own verification tasks 5.2 and 5.3.
//!
//! The per-reader tests exercise one step each; these run the full path a user
//! takes - scan a real directory of files, import what it offered, and read
//! the summary - against a Knot that already holds personas, agents and
//! workspaces, which is the state the additive guarantee is actually about.

use std::fs;

use tempfile::TempDir;
use uuid::Uuid;

use super::result::UnreadableReason;
use super::skwad::SkwadSource;
use super::subagents::claude::definitions_in;
use super::{import_definitions, import_workspaces};
use crate::settings::{Persona, PersonaState, PersonaType, SavedAgent, Settings, Workspace};

fn definition(name: &str) -> String {
    format!("---\nname: {name}\ndescription: whatever\n---\n\nInstructions for {name}.\n")
}

fn write(dir: &TempDir, file: &str, contents: &str) {
    fs::write(dir.path().join(file), contents).expect("fixture written");
}

/// A Knot that is already in use: the state an import must not disturb.
fn populated_store(dir: &TempDir) -> Settings {
    let mut settings = Settings::with_store_path(dir.path().join("settings.json"));
    settings.personas
            .push(Persona { id:           Uuid::new_v4(),
                            name:         "my-persona".into(),
                            instructions: "Mine.".into(),
                            persona_type: PersonaType::User,
                            state:        PersonaState::Enabled, });
    settings.saved_agents
            .push(SavedAgent::new(Uuid::new_v4(), "my-agent", None, "/mine"));
    settings.saved_workspaces
            .push(Workspace { id:                    Uuid::new_v4(),
                              name:                  "Mine".into(),
                              color_hex:             "#46A857".into(),
                              agent_ids:             Vec::new(),
                              layout_mode:           "single".into(),
                              active_agent_ids:      Vec::new(),
                              focused_pane_index:    0,
                              split_ratio:           0.5,
                              split_ratio_secondary: None,
                              show_dashboard:        None,
                              is_detached:           None,
                              window_bounds:         None, });
    settings
}

/// Task 5.3: one malformed file among several valid ones. The valid ones
/// import and the bad one is named.
#[test]
fn a_malformed_definition_among_valid_ones_is_named_and_the_rest_import() {
    let fixtures = TempDir::new().expect("temp dir");
    write(&fixtures, "one.md", &definition("one"));
    write(&fixtures, "two.md", &definition("two"));
    write(&fixtures, "three.md", &definition("three"));
    write(&fixtures, "broken.md", "no frontmatter here\n");
    let store_dir = TempDir::new().expect("temp dir");
    let mut settings = populated_store(&store_dir);

    let scan = definitions_in(fixtures.path());
    let mut result = import_definitions(&mut settings, &scan.definitions).expect("import succeeds");
    result.unreadable.extend(scan.unreadable);

    assert_eq!(result.added,
               ["one", "three", "two"],
               "every readable definition imported");
    assert_eq!(result.unreadable.len(), 1);
    assert_eq!(result.unreadable[0].name, "broken.md",
               "the bad one is named, not just counted");
    assert_eq!(result.unreadable[0].reason, UnreadableReason::NoFrontmatter);

    let rendered = result.summary_lines().join("\n");
    assert!(rendered.contains("broken.md"), "the summary names it too");
}

/// Task 5.2: with a Knot that already holds personas, agents and workspaces,
/// run both imports and verify nothing pre-existing changed.
#[test]
fn both_imports_leave_everything_that_was_already_there_alone() {
    let fixtures = TempDir::new().expect("temp dir");
    write(&fixtures, "one.md", &definition("one"));
    let store_dir = TempDir::new().expect("temp dir");
    let mut settings = populated_store(&store_dir);
    let before = settings.clone();

    let imported_persona = Persona { id:           Uuid::new_v4(),
                                     name:         "Terse".into(),
                                     instructions: "Be brief.".into(),
                                     persona_type: PersonaType::User,
                                     state:        PersonaState::Enabled, };
    let mut imported_agent = SavedAgent::new(Uuid::new_v4(), "theirs", None, "/theirs");
    imported_agent.persona_id = Some(imported_persona.id);
    let imported_workspace = Workspace { agent_ids: vec![imported_agent.id],
                                         ..before.saved_workspaces[0].clone() };
    let workspace_id = Uuid::new_v4();
    let imported_workspace = Workspace { id: workspace_id,
                                         ..imported_workspace };
    let source = SkwadSource { workspaces:   vec![imported_workspace],
                               agents:       vec![imported_agent],
                               personas:     vec![imported_persona],
                               bench_agents: Vec::new(),
                               unreadable:   Vec::new(), };

    let scan = definitions_in(fixtures.path());
    import_definitions(&mut settings, &scan.definitions).expect("persona import succeeds");
    import_workspaces(&mut settings, &source, &[workspace_id]).expect("workspace import succeeds");

    // Name, contents and ordering: each pre-existing record is still the first
    // of its collection, byte for byte.
    assert_eq!(&settings.personas[..1], &before.personas[..]);
    assert_eq!(&settings.saved_agents[..1], &before.saved_agents[..]);
    assert_eq!(&settings.saved_workspaces[..1],
               &before.saved_workspaces[..]);
    assert!(settings.personas.len() > before.personas.len(),
            "and the import did add something");
}

/// The headline guarantee across both sources at once: run everything twice,
/// and the second run changes nothing.
#[test]
fn running_both_imports_twice_changes_nothing_the_second_time() {
    let fixtures = TempDir::new().expect("temp dir");
    write(&fixtures, "one.md", &definition("one"));
    write(&fixtures, "two.md", &definition("two"));
    let store_dir = TempDir::new().expect("temp dir");
    let mut settings = populated_store(&store_dir);

    let persona = Persona { id:           Uuid::new_v4(),
                            name:         "Terse".into(),
                            instructions: "Be brief.".into(),
                            persona_type: PersonaType::User,
                            state:        PersonaState::Enabled, };
    let mut agent = SavedAgent::new(Uuid::new_v4(), "theirs", None, "/theirs");
    agent.persona_id = Some(persona.id);
    let workspace = Workspace { id:                    Uuid::new_v4(),
                                name:                  "Theirs".into(),
                                color_hex:             "#46A857".into(),
                                agent_ids:             vec![agent.id],
                                layout_mode:           "single".into(),
                                active_agent_ids:      Vec::new(),
                                focused_pane_index:    0,
                                split_ratio:           0.5,
                                split_ratio_secondary: None,
                                show_dashboard:        None,
                                is_detached:           None,
                                window_bounds:         None, };
    let workspace_id = workspace.id;
    let source = SkwadSource { workspaces:   vec![workspace],
                               agents:       vec![agent],
                               personas:     vec![persona],
                               bench_agents: Vec::new(),
                               unreadable:   Vec::new(), };
    let scan = definitions_in(fixtures.path());

    import_definitions(&mut settings, &scan.definitions).expect("first persona import");
    import_workspaces(&mut settings, &source, &[workspace_id]).expect("first workspace import");
    let after_first = settings.clone();

    let personas_again =
        import_definitions(&mut settings, &scan.definitions).expect("second persona import");
    let workspaces_again =
        import_workspaces(&mut settings, &source, &[workspace_id]).expect("second workspace import");

    assert!(personas_again.added.is_empty(), "no persona added twice");
    assert!(workspaces_again.added.is_empty(),
            "no workspace added twice");
    assert_eq!(personas_again.skipped, ["one", "two"]);
    assert_eq!(workspaces_again.skipped, ["Theirs"]);
    assert_eq!(settings.personas, after_first.personas);
    assert_eq!(settings.saved_agents, after_first.saved_agents);
    assert_eq!(settings.saved_workspaces, after_first.saved_workspaces);
}

/// A source that is not installed contributes nothing and raises no error,
/// which is what a machine without Skwad or without Claude Code looks like.
#[test]
fn a_source_that_is_not_installed_contributes_nothing() {
    let store_dir = TempDir::new().expect("temp dir");
    let mut settings = populated_store(&store_dir);
    let before = settings.clone();

    let scan = definitions_in(&store_dir.path().join("no-such-directory"));
    let personas = import_definitions(&mut settings, &scan.definitions).expect("no error");
    let workspaces =
        import_workspaces(&mut settings, &SkwadSource::default(), &[]).expect("no error");

    assert!(scan.is_empty());
    assert!(personas.is_empty());
    assert!(workspaces.is_empty());
    assert_eq!(settings.personas, before.personas);
}
