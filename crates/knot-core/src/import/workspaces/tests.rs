//! Tests for importing Skwad workspaces.
//!
//! One case per behaviour the contract names: a workspace arrives with its
//! agents and their personas, a bench template comes with the folder, an agent
//! whose persona is missing arrives without one, and an id Knot already holds
//! is skipped.

use tempfile::TempDir;
use uuid::Uuid;

use super::import_workspaces;
use crate::import::result::UnreadableReason;
use crate::import::skwad::SkwadSource;
use crate::settings::{
    BenchAgent, Persona, PersonaState, PersonaType, SavedAgent, Settings, SharedSettings, Workspace,
};

const FOLDER: &str = "/Users/someone/WIP/thing";

fn store(dir: &TempDir) -> Settings {
    Settings::with_store_root(dir.path())
}

fn workspace(name: &str, agent_ids: Vec<Uuid>) -> Workspace {
    Workspace { id: Uuid::new_v4(),
                name: name.into(),
                color_hex: "#46A857".into(),
                agent_ids }
}

fn agent(name: &str, persona_id: Option<Uuid>) -> SavedAgent {
    let mut agent = SavedAgent::new(Uuid::new_v4(), name, None, FOLDER);
    agent.persona_id = persona_id;
    agent
}

fn persona(name: &str) -> Persona {
    Persona { id:           Uuid::new_v4(),
              name:         name.into(),
              instructions: "Be brief.".into(),
              persona_type: PersonaType::User,
              state:        PersonaState::Enabled, }
}

/// A source holding one workspace of three agents, two of which share a
/// persona - the contract's own example.
fn source_with_three_agents() -> (SkwadSource, Uuid) {
    let shared = persona("Terse");
    let agents = vec![agent("first", Some(shared.id)),
                      agent("second", Some(shared.id)),
                      agent("third", None)];
    let ws = workspace("WIP", agents.iter().map(|a| a.id).collect());
    let id = ws.id;
    (SkwadSource { workspaces: vec![ws],
                   agents,
                   personas: vec![shared],
                   bench_agents: Vec::new(),
                   unreadable: Vec::new() },
     id)
}

#[test]
fn a_workspace_arrives_with_its_agents_and_their_persona() {
    let dir = TempDir::new().expect("temp dir");
    let settings = store(&dir);
    let (source, id) = source_with_three_agents();
    let shared = SharedSettings::new(settings);

    import_workspaces(&shared, &source, &[id]).expect("import succeeds");

    let settings = shared.read();
    assert_eq!(settings.saved_workspaces.len(), 1);
    assert_eq!(settings.saved_agents.len(), 3);
    assert_eq!(settings.personas.len(),
               1,
               "the shared persona arrives once");
}

/// The workspace's agent order is the user's arrangement; it must survive.
#[test]
fn the_agents_keep_the_order_the_workspace_lists_them_in() {
    let dir = TempDir::new().expect("temp dir");
    let settings = store(&dir);
    let (source, id) = source_with_three_agents();
    let shared = SharedSettings::new(settings);

    import_workspaces(&shared, &source, &[id]).expect("import succeeds");

    let settings = shared.read();
    let names: Vec<&str> = settings.saved_agents
                                   .iter()
                                   .map(|a| a.name.as_str())
                                   .collect();
    assert_eq!(names, ["first", "second", "third"]);
    assert_eq!(settings.saved_workspaces[0].agent_ids,
               settings.saved_agents
                       .iter()
                       .map(|a| a.id)
                       .collect::<Vec<_>>());
}

#[test]
fn a_bench_template_for_the_folder_comes_with_it() {
    let dir = TempDir::new().expect("temp dir");
    let settings = store(&dir);
    let (mut source, id) = source_with_three_agents();
    source.bench_agents = vec![BenchAgent::new(Uuid::new_v4(), "On-call", None, FOLDER)];
    let shared = SharedSettings::new(settings);

    import_workspaces(&shared, &source, &[id]).expect("import succeeds");

    let settings = shared.read();
    assert_eq!(settings.bench_agents.len(), 1);
    assert_eq!(settings.bench_agents[0].name, "On-call");
}

#[test]
fn a_bench_template_for_another_folder_is_left_behind() {
    let dir = TempDir::new().expect("temp dir");
    let settings = store(&dir);
    let (mut source, id) = source_with_three_agents();
    source.bench_agents =
        vec![BenchAgent::new(Uuid::new_v4(), "Elsewhere", None, "/somewhere/else")];
    let shared = SharedSettings::new(settings);

    import_workspaces(&shared, &source, &[id]).expect("import succeeds");

    let settings = shared.read();
    assert!(settings.bench_agents.is_empty(),
            "only the imported agents' folders come across");
}

/// The contract: imported without a persona rather than skipped, and the lost
/// reference named.
#[test]
fn an_agent_whose_persona_is_missing_arrives_without_one() {
    let dir = TempDir::new().expect("temp dir");
    let settings = store(&dir);
    let orphan = agent("orphan", Some(Uuid::new_v4()));
    let ws = workspace("WIP", vec![orphan.id]);
    let id = ws.id;
    let source = SkwadSource { workspaces: vec![ws],
                               agents: vec![orphan],
                               ..SkwadSource::default() };
    let shared = SharedSettings::new(settings);

    let result = import_workspaces(&shared, &source, &[id]).expect("import succeeds");

    let settings = shared.read();
    assert_eq!(settings.saved_agents.len(), 1, "the agent still arrives");
    assert!(result.added.contains(&"orphan".to_string()));
    assert_eq!(result.unreadable.len(), 1);
    assert_eq!(result.unreadable[0].name, "orphan");
    assert_eq!(result.unreadable[0].reason,
               UnreadableReason::MissingPersona);
}

/// Skwad's ids are exact, which is what makes re-importing safe.
#[test]
fn a_workspace_already_held_is_skipped() {
    let dir = TempDir::new().expect("temp dir");
    let settings = store(&dir);
    let (source, id) = source_with_three_agents();
    let shared = SharedSettings::new(settings);

    import_workspaces(&shared, &source, &[id]).expect("first import");
    let second = import_workspaces(&shared, &source, &[id]).expect("second import");

    let settings = shared.read();
    assert!(second.added.is_empty(), "nothing new the second time");
    assert_eq!(second.skipped, ["WIP"]);
    assert_eq!(settings.saved_workspaces.len(), 1);
    assert_eq!(settings.saved_agents.len(), 3, "no duplicated agents");
    assert_eq!(settings.personas.len(), 1, "no duplicated persona");
}

/// Re-importing must not disturb what the user has done since - an agent added
/// to Knot's copy of the workspace stays exactly where it was.
#[test]
fn re_importing_leaves_agents_added_since_untouched() {
    let dir = TempDir::new().expect("temp dir");
    let shared = SharedSettings::new(store(&dir));
    let (source, id) = source_with_three_agents();
    import_workspaces(&shared, &source, &[id]).expect("first import");
    // The user adds an agent of their own after the first import, which is
    // what "added since" means. It goes through the surface, as the app's own
    // roster writes do.
    let added_since = SavedAgent::new(Uuid::new_v4(), "added-since", None, FOLDER);
    let before = shared.write(|settings| {
                           settings.saved_agents.push(added_since.clone());
                           settings.saved_workspaces[0].agent_ids.push(added_since.id);
                       });

    import_workspaces(&shared, &source, &[id]).expect("second import");

    let settings = shared.read();
    assert_eq!(settings.saved_agents, before.saved_agents);
    assert_eq!(settings.saved_workspaces, before.saved_workspaces);
}

#[test]
fn a_persona_knot_already_holds_is_not_duplicated() {
    let dir = TempDir::new().expect("temp dir");
    let mut settings = store(&dir);
    let (source, id) = source_with_three_agents();
    settings.personas.push(source.personas[0].clone());
    let shared = SharedSettings::new(settings);

    let result = import_workspaces(&shared, &source, &[id]).expect("import succeeds");

    let settings = shared.read();
    assert_eq!(settings.personas.len(), 1);
    assert!(result.skipped.contains(&"Terse".to_string()));
}

#[test]
fn an_unselected_workspace_is_left_alone() {
    let dir = TempDir::new().expect("temp dir");
    let settings = store(&dir);
    let (mut source, id) = source_with_three_agents();
    let other = workspace("Other", Vec::new());
    source.workspaces.push(other);
    let shared = SharedSettings::new(settings);

    import_workspaces(&shared, &source, &[id]).expect("import succeeds");

    let settings = shared.read();
    assert_eq!(settings.saved_workspaces.len(), 1);
    assert_eq!(settings.saved_workspaces[0].name, "WIP");
}

#[test]
fn an_id_the_source_does_not_hold_is_ignored() {
    let dir = TempDir::new().expect("temp dir");
    let settings = store(&dir);
    let (source, _) = source_with_three_agents();
    let shared = SharedSettings::new(settings);

    let result = import_workspaces(&shared, &source, &[Uuid::new_v4()]).expect("import succeeds");

    let settings = shared.read();
    assert!(result.is_empty());
    assert!(settings.saved_workspaces.is_empty());
}

/// `data-import`: an import never removes or reorders what the user has.
#[test]
fn what_knot_already_held_is_still_there_afterwards() {
    let dir = TempDir::new().expect("temp dir");
    let mut settings = store(&dir);
    settings.personas.push(persona("mine"));
    settings.saved_agents
            .push(SavedAgent::new(Uuid::new_v4(), "my-agent", None, "/mine"));
    settings.saved_workspaces
            .push(workspace("My Workspace", Vec::new()));
    let before = settings.clone();
    let shared = SharedSettings::new(settings);
    let (source, id) = source_with_three_agents();

    import_workspaces(&shared, &source, &[id]).expect("import succeeds");

    let settings = shared.read();
    assert_eq!(&settings.personas[..1], &before.personas[..]);
    assert_eq!(&settings.saved_agents[..1], &before.saved_agents[..]);
    assert_eq!(&settings.saved_workspaces[..1],
               &before.saved_workspaces[..]);
}

#[test]
fn an_imported_workspace_is_persisted() {
    let dir = TempDir::new().expect("temp dir");
    let settings = Settings::with_store_root(dir.path());
    let (source, id) = source_with_three_agents();
    let shared = SharedSettings::new(settings);

    import_workspaces(&shared, &source, &[id]).expect("import succeeds");

    // Reads from disk, not from the surface: the point is that the records
    // survive a restart, so an in-memory assertion would not test it.
    let reloaded = Settings::load_from_root(dir.path()).expect("reload");
    assert_eq!(reloaded.saved_workspaces.len(), 1);
    assert_eq!(reloaded.saved_agents.len(), 3);
}

/// The write half of #238 at this site: the import window handed over the
/// copy it loaded when it opened, and the import wrote that copy back over
/// every document - so a preference changed while the window stood open was
/// reverted by the next import.
///
/// `openspec/specs/settings-persistence/spec.md`, "A write preserves values
/// written elsewhere" - scenario "An import does not revert a preference".
#[test]
fn an_import_does_not_revert_a_preference_changed_while_it_was_open() {
    let dir = TempDir::new().expect("temp dir");
    let shared = SharedSettings::new(store(&dir));
    let (source, id) = source_with_three_agents();
    // The import window opens and reads the surface, as it does today.
    let held_since_the_window_opened = shared.read();

    // The settings window changes a preference and writes it.
    shared.write(|settings| settings.mcp_server_port = 9000);
    shared.read()
          .persist_preferences()
          .expect("the settings window writes preferences");

    // Only now does the user run the import.
    import_workspaces(&shared, &source, &[id]).expect("import succeeds");

    assert_ne!(held_since_the_window_opened.mcp_server_port, 9000,
               "the window's early read must genuinely be stale, or this proves nothing");
    assert_eq!(shared.read().mcp_server_port,
               9000,
               "the import must not revert the preference");
    let reloaded = Settings::load_from_root(dir.path()).expect("reload");
    assert_eq!(reloaded.mcp_server_port, 9000,
               "and must not revert it on disk either");
    assert_eq!(reloaded.saved_workspaces.len(),
               1,
               "while still importing the workspace");
    assert_eq!(reloaded.saved_agents.len(), 3, "and its agents");
}

/// An import writes the collection documents it added to and no others. The
/// preferences document is the one that mattered - writing it is what
/// reverted settings - but a personas document rewritten by a workspace-only
/// import is the same mistake, smaller.
///
/// `openspec/specs/settings-persistence/spec.md`, "A write touches only the
/// document it belongs to".
#[test]
fn an_import_leaves_the_documents_it_did_not_touch_alone() {
    let dir = TempDir::new().expect("temp dir");
    let shared = SharedSettings::new(store(&dir));
    // An agent with no persona, whose folder has no bench template, so the
    // import reaches the roster documents and nothing else.
    let lone = agent("lone", None);
    let ws = workspace("WIP", vec![lone.id]);
    let id = ws.id;
    let source = SkwadSource { workspaces:   vec![ws],
                               agents:       vec![lone],
                               personas:     Vec::new(),
                               bench_agents: Vec::new(),
                               unreadable:   Vec::new(), };

    import_workspaces(&shared, &source, &[id]).expect("import succeeds");

    let root = dir.path();
    assert!(root.join("agents.json").exists(),
            "the agents document is written");
    assert!(root.join("workspaces.json").exists(),
            "and the workspaces document");
    assert!(!root.join("personas.json").exists(),
            "but not the personas document, which this import never added to");
    assert!(!root.join("bench.json").exists(), "nor the bench document");
    assert!(!root.join("preferences.json").exists(),
            "and above all not the preferences document");
}
