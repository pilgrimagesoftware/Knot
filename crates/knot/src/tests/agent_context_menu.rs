//! One agent row's context menu: which entries appear, in what order, and
//! where the dividers fall.
//!
//! Asserted against the Swift reference's ordering, since the menu is a
//! port of it - `openspec/specs/agent-list-ui/spec.md`.

use std::path::PathBuf;

use uuid::Uuid;

use crate::agent_editor::created_agent_type;
use crate::agent_editor::persona_choices;
use crate::app_state::AgentMenuEntry;
use crate::app_state::AgentMenuFacts;
use crate::app_state::agent_context_menu_entries;
use crate::tests::menu_labels;
use crate::tests::workspace;
use crate::workspace_window::agent_menu_facts;

#[test]
fn agent_context_menu_matches_the_swift_reference_order_for_a_full_menu() {
    let facts = AgentMenuFacts { is_companion:         false,
                                 is_shell:             false,
                                 has_move_targets:     true,
                                 has_markdown_history: true,
                                 is_running:           true, };
    assert_eq!(menu_labels(facts),
               vec!["New Companion…",
                    "New Shell Companion",
                    "-",
                    "Edit Agent…",
                    "Fork Agent",
                    "Duplicate Agent",
                    "-",
                    "Move to Workspace",
                    "Save to Bench",
                    "-",
                    "Open In…",
                    "Markdown Files",
                    "-",
                    "Register Agent",
                    "Deactivate",
                    "Restart Agent",
                    "Remove Agent"]);
}

#[test]
fn agent_context_menu_omits_companion_actions_for_companions() {
    let facts = AgentMenuFacts { is_companion:         true,
                                 is_shell:             true,
                                 has_move_targets:     true,
                                 has_markdown_history: false,
                                 is_running:           false, };
    assert_eq!(menu_labels(facts),
               vec!["Edit Agent…", "-", "Open In…", "-", "Remove Agent"]);
}

/// The store-reading half of the menu: which workspaces an agent can move
/// to, and whether it has markdown history. A detached workspace lives in
/// its own window and is not a move target; neither is the agent's own.
#[test]
fn agent_menu_facts_exclude_the_agents_own_workspace_and_detached_ones() {
    let mut store = knot_agents::AgentStore::new();
    let here = workspace("Here");
    let there = workspace("There");
    let mut detached = workspace("Detached");
    detached.is_detached = Some(true);
    let (here_id, there_id) = (here.id, there.id);
    store.add_workspace(here);
    store.add_workspace(there);
    store.add_workspace(detached);
    store.set_current_workspace(here_id);
    let id = store.create("~/alpha", knot_agents::CreateOptions::default());

    let (facts, move_targets, history) = agent_menu_facts(&store, id);

    assert!(facts.has_move_targets);
    assert_eq!(move_targets.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
               vec![there_id],
               "only the other attached workspace is a target");
    assert!(!facts.has_markdown_history);
    assert!(history.is_empty());
    assert!(!facts.is_companion);
    assert!(!facts.is_shell);
}

#[test]
fn agent_menu_facts_report_a_companion_and_its_markdown_history() {
    let mut store = knot_agents::AgentStore::new();
    let ws = workspace("One");
    let ws_id = ws.id;
    store.add_workspace(ws);
    store.set_current_workspace(ws_id);
    let owner = store.create("~/alpha", knot_agents::CreateOptions::default());
    let companion = store.create_shell_companion(owner).unwrap();
    store.set_markdown_panel(companion, PathBuf::from("/tmp/plan.md"), false)
         .unwrap();

    let (facts, move_targets, history) = agent_menu_facts(&store, companion);

    assert!(facts.is_companion);
    assert!(facts.is_shell);
    assert!(!facts.has_move_targets,
            "the only workspace is the agent's own");
    assert!(move_targets.is_empty());
    assert!(facts.has_markdown_history);
    assert_eq!(history, vec![PathBuf::from("/tmp/plan.md")]);
}

/// A menu opened on an agent that is already gone must not invent facts
/// for it - the row can outlive the agent by a frame.
#[test]
fn agent_menu_facts_are_empty_for_a_missing_agent() {
    let store = knot_agents::AgentStore::new();
    let (facts, move_targets, history) = agent_menu_facts(&store, Uuid::new_v4());
    assert_eq!(facts, AgentMenuFacts::default());
    assert!(move_targets.is_empty());
    assert!(history.is_empty());
}

/// "New Companion…" routes through the editor, which has an agent-type
/// picker; a companion created as anything but `shell` is one the MCP
/// `create-agent` tool would refuse and `create_shell_companion` cannot
/// produce, so the editor must not be able to make one either.
#[test]
fn a_companion_is_always_created_as_a_shell_agent() {
    assert_eq!(created_agent_type(true, "claude"), "shell");
    assert_eq!(created_agent_type(true, "shell"), "shell");
    assert_eq!(created_agent_type(false, "claude"), "claude");
}

/// The editor's persona picker offers the *active* list, not the stored
/// one: `openspec/specs/personas/spec.md` - "Active versus stored
/// personas" - reserves the stored list for persistence and requires the
/// active list for selection, so a soft-deleted system persona must not be
/// assignable here. The active list is also the sorted one, which is the
/// order the picker should read in.
#[test]
fn the_persona_picker_offers_active_personas_only_in_sorted_order() {
    let mut settings = knot_core::Settings::default();
    settings.personas = vec![knot_core::Persona { id:           Uuid::new_v4(),
                                                  name:         "beta".to_string(),
                                                  instructions: String::new(),
                                                  persona_type: knot_core::PersonaType::User,
                                                  state:        knot_core::PersonaState::Enabled, },
                             knot_core::Persona { id:           Uuid::new_v4(),
                                                  name:         "Alpha".to_string(),
                                                  instructions: String::new(),
                                                  persona_type: knot_core::PersonaType::User,
                                                  state:        knot_core::PersonaState::Enabled, },
                             knot_core::Persona { id:           Uuid::new_v4(),
                                                  name:         "Gone".to_string(),
                                                  instructions: String::new(),
                                                  persona_type: knot_core::PersonaType::System,
                                                  state:        knot_core::PersonaState::Deleted, },];

    let names: Vec<String> = persona_choices(&settings).into_iter()
                                                       .map(|persona| persona.name)
                                                       .collect();

    assert_eq!(names, vec!["Alpha".to_string(), "beta".to_string()]);
}

/// 3.3: Deactivate is there only while there is a session to stop, and it
/// sits immediately above Restart Agent when it is.
#[test]
fn agent_context_menu_offers_deactivate_only_for_a_running_agent() {
    let stopped = menu_labels(AgentMenuFacts::default());
    assert!(!stopped.contains(&"Deactivate"),
            "a passive agent that never started has nothing to stop");

    let running = menu_labels(AgentMenuFacts { is_running: true,
                                               ..Default::default() });
    let deactivate = running.iter().position(|label| *label == "Deactivate");
    let restart = running.iter().position(|label| *label == "Restart Agent");
    assert_eq!(deactivate.zip(restart).map(|(d, r)| r == d + 1),
               Some(true),
               "Deactivate sits immediately above Restart Agent: {running:?}");
}

/// A running companion can be stopped on its own, even though it cannot be
/// restarted independently of its owner.
#[test]
fn agent_context_menu_offers_deactivate_for_a_running_companion() {
    let labels = menu_labels(AgentMenuFacts { is_companion: true,
                                              is_shell: true,
                                              is_running: true,
                                              ..Default::default() });
    assert!(labels.contains(&"Deactivate"));
    assert!(!labels.contains(&"Restart Agent"));
}

/// `agent_menu_facts` reads liveness from the store, so the menu reflects
/// what is running at the moment it opens.
#[test]
fn agent_menu_facts_report_whether_the_agent_is_running() {
    let mut store = knot_agents::AgentStore::new();
    let ws = workspace("One");
    let ws_id = ws.id;
    store.add_workspace(ws);
    store.set_current_workspace(ws_id);
    let id = store.create("~/alpha", knot_agents::CreateOptions::default());

    assert!(!agent_menu_facts(&store, id).0.is_running);

    store.set_activated(id, true);
    assert!(agent_menu_facts(&store, id).0.is_running);
}

#[test]
fn agent_context_menu_hides_register_for_a_shell_agent() {
    let facts = AgentMenuFacts { is_shell: true,
                                 ..Default::default() };
    assert!(!menu_labels(facts).contains(&"Register Agent"));
    assert!(menu_labels(AgentMenuFacts { is_shell: false,
                                         ..Default::default() }).contains(&"Register Agent"));
}

#[test]
fn agent_context_menu_hides_move_to_workspace_without_a_target() {
    assert!(!menu_labels(AgentMenuFacts::default()).contains(&"Move to Workspace"));
    assert!(menu_labels(AgentMenuFacts { has_move_targets: true,
                                         ..Default::default() }).contains(&"Move to Workspace"));
}

#[test]
fn agent_context_menu_hides_markdown_files_without_history() {
    assert!(!menu_labels(AgentMenuFacts::default()).contains(&"Markdown Files"));
    assert!(menu_labels(AgentMenuFacts { has_markdown_history: true,
                                         ..Default::default() }).contains(&"Markdown Files"));
}

/// Every hidden group must take its divider with it - the menu can never
/// open on a separator, end on one, or show two in a row.
#[test]
fn agent_context_menu_never_emits_a_stray_divider() {
    for is_companion in [false, true] {
        for is_shell in [false, true] {
            for has_move_targets in [false, true] {
                for has_markdown_history in [false, true] {
                    for is_running in [false, true] {
                        let facts = AgentMenuFacts { is_companion,
                                                     is_shell,
                                                     has_move_targets,
                                                     has_markdown_history,
                                                     is_running };
                        let entries = agent_context_menu_entries(facts);
                        assert_ne!(entries.first(),
                                   Some(&AgentMenuEntry::Separator),
                                   "{facts:?}");
                        assert_ne!(entries.last(),
                                   Some(&AgentMenuEntry::Separator),
                                   "{facts:?}");
                        assert!(!entries.windows(2).any(|pair| pair
                                                               == [AgentMenuEntry::Separator,
                                                                   AgentMenuEntry::Separator]),
                                "{facts:?}");
                    }
                }
            }
        }
    }
}
