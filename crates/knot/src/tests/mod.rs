mod about_dialog;
mod agents_menu;

use knot_core::Workspace;

use super::*;

fn workspace(name: &str) -> Workspace {
    Workspace { id:                    Uuid::new_v4(),
                name:                  name.to_string(),
                color_hex:             "#123456".to_string(),
                agent_ids:             Vec::new(),
                layout_mode:           "single".to_string(),
                active_agent_ids:      Vec::new(),
                focused_pane_index:    0,
                split_ratio:           0.5,
                split_ratio_secondary: None,
                show_dashboard:        None,
                is_detached:           None,
                window_bounds:         None, }
}

/// The labels a given set of facts produces, separators rendered as
/// `"-"` so ordering *and* divider placement are both asserted.
fn menu_labels(facts: AgentMenuFacts) -> Vec<&'static str> {
    agent_context_menu_entries(facts).into_iter()
                                     .map(|entry| entry.label().unwrap_or("-"))
                                     .collect()
}

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

#[test]
fn empty_store_has_no_rows() {
    let store = knot_agents::AgentStore::new();
    let model = layout_model(&store, None, &[], &BTreeMap::new());
    assert!(model.workspace_rows.is_empty());
    assert!(model.selected_agent_rows.is_empty());
}

#[test]
fn selected_workspace_marks_and_filters_rows() {
    let mut store = knot_agents::AgentStore::new();
    let ws1 = workspace("One");
    let ws2 = workspace("Two");
    store.add_workspace(ws1.clone());
    store.add_workspace(ws2.clone());

    store.set_current_workspace(ws1.id);
    store.create("~/alpha", knot_agents::CreateOptions::default());
    store.create("~/beta", knot_agents::CreateOptions::default());

    store.set_current_workspace(ws2.id);
    store.create("~/gamma", knot_agents::CreateOptions::default());

    store.set_current_workspace(ws1.id);
    let model = layout_model(&store, None, &[], &BTreeMap::new());

    assert_eq!(model.workspace_rows.len(), 2);
    assert!(model.workspace_rows
                 .iter()
                 .find(|r| r.id == ws1.id)
                 .unwrap()
                 .selected);
    assert!(!model.workspace_rows
                  .iter()
                  .find(|r| r.id == ws2.id)
                  .unwrap()
                  .selected);

    let names = model.selected_agent_rows
                     .iter()
                     .map(|r| r.name.as_str())
                     .collect::<Vec<_>>();
    assert_eq!(names, vec!["alpha", "beta"]);
    assert!(!names.contains(&"gamma"));
    let gamma_id = store.agents()
                        .iter()
                        .find(|agent| agent.name == "gamma")
                        .map(|agent| agent.id)
                        .unwrap();
    assert_eq!(agent_selection_for_workspace(&store, ws2.id),
               Some(gamma_id));
}

#[test]
fn missing_agent_ids_are_skipped() {
    let mut store = knot_agents::AgentStore::new();
    let mut ws = workspace("One");
    ws.agent_ids.push(Uuid::new_v4());
    store.add_workspace(ws.clone());
    store.set_current_workspace(ws.id);
    store.create("~/alpha", knot_agents::CreateOptions::default());

    let model = layout_model(&store, None, &[], &BTreeMap::new());
    assert_eq!(model.selected_agent_rows.len(), 1);
    assert_eq!(model.selected_agent_rows[0].name, "alpha");
}

#[test]
fn agent_selection_marks_and_tracks_attach_state() {
    let mut store = knot_agents::AgentStore::new();
    let ws = workspace("One");
    store.add_workspace(ws.clone());
    store.set_current_workspace(ws.id);
    let alpha_id = store.create("~/alpha", knot_agents::CreateOptions::default());
    store.create("~/beta", knot_agents::CreateOptions::default());

    let model = layout_model(&store, Some(alpha_id), &[], &BTreeMap::new());
    let alpha = model.selected_agent_rows
                     .iter()
                     .find(|row| row.id == alpha_id)
                     .unwrap();
    assert!(alpha.selected);
    assert!(!alpha.attached);
    assert_eq!(alpha.state, knot_agents::AgentState::Idle);
    let beta = model.selected_agent_rows
                    .iter()
                    .find(|row| row.id != alpha_id)
                    .unwrap();
    assert!(!beta.selected);

    let model = layout_model(&store, Some(alpha_id), &[alpha_id], &BTreeMap::new());
    assert!(model.selected_agent_rows
                 .iter()
                 .find(|row| row.id == alpha_id)
                 .unwrap()
                 .attached);
}

/// `agent-lifecycle`'s exit-driven removal is scoped by which agents own
/// a terminal process: a shell agent's exiting shell removes it, while an
/// ACP agent's adapter exiting does not (it has no PTY here at all).
#[test]
fn only_shell_agents_run_a_terminal_process() {
    assert!(runs_a_terminal_process("shell"));
    for agent_type in ["claude", "codex", "opencode", "gemini", "copilot"] {
        assert!(!runs_a_terminal_process(agent_type),
                "{agent_type} must not get a PTY under ACP-only launch");
    }
}

#[test]
fn state_label_matches_the_swift_reference_strings() {
    assert_eq!(state_label(knot_agents::AgentState::Idle), "Idle");
    assert_eq!(state_label(knot_agents::AgentState::Running), "Working");
    assert_eq!(state_label(knot_agents::AgentState::Input),
               "Awaiting input");
    assert_eq!(state_label(knot_agents::AgentState::Error), "Error");
}

#[test]
fn layout_model_carries_agent_state_into_rows() {
    let mut store = knot_agents::AgentStore::new();
    let ws = workspace("One");
    store.add_workspace(ws.clone());
    store.set_current_workspace(ws.id);
    let id = store.create("~/alpha", knot_agents::CreateOptions::default());
    store.set_state(id, knot_agents::AgentState::Input);

    let model = layout_model(&store, None, &[], &BTreeMap::new());
    assert_eq!(model.selected_agent_rows[0].state,
               knot_agents::AgentState::Input);
}

#[test]
fn command_to_send_trims_input_and_rejects_empty_commands() {
    assert_eq!(command_to_send("  cargo test  "), Some("cargo test"));
    assert_eq!(command_to_send("\t\n"), None);
}

#[test]
fn stale_session_ids_excludes_live_agents() {
    let live = Uuid::new_v4();
    let stale = Uuid::new_v4();
    let live_ids = BTreeSet::from([live]);

    assert_eq!(stale_session_ids(&[live, stale], &live_ids), vec![stale]);
}

#[test]
fn delivery_notice_names_the_last_known_recipient_and_counts_events() {
    let mut store = knot_agents::AgentStore::new();
    let first = store.create("~/first", knot_agents::CreateOptions::default());
    let second = store.create("~/second", knot_agents::CreateOptions::default());
    let events = vec![DeliveryEvent { agent_id:   first,
                                      message_id: Uuid::new_v4(), },
                      DeliveryEvent { agent_id:   second,
                                      message_id: Uuid::new_v4(), },
                      DeliveryEvent { agent_id:   second,
                                      message_id: Uuid::new_v4(), },];

    assert_eq!(delivery_notice(&events, store.agents()),
               Some(DeliveryNotice { recipient_name: "second".to_string(),
                                     count:          2, }));
    assert_eq!(delivery_notice(&[], store.agents()), None);
}

#[test]
fn delivery_notice_ignores_unknown_recipients() {
    let store = knot_agents::AgentStore::new();
    let events = [DeliveryEvent { agent_id:   Uuid::new_v4(),
                                  message_id: Uuid::new_v4(), }];

    assert_eq!(delivery_notice(&events, store.agents()), None);
}

#[test]
fn unread_counts_snapshot_includes_zero_and_ignores_other_agents() {
    let mut messages = knot_messaging::MessageStore::new();
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    let other = Uuid::new_v4();
    messages.add(knot_messaging::Message::new(other, first, "one"));
    messages.add(knot_messaging::Message::new(other, first, "two"));
    messages.add(knot_messaging::Message::new(other, other, "unrelated"));

    let counts = unread_counts_snapshot(&messages, &[first, second]);

    assert_eq!(counts.get(&first), Some(&2));
    assert_eq!(counts.get(&second), Some(&0));
    assert!(!counts.contains_key(&other));
}

#[test]
fn layout_model_carries_unread_count_into_agent_rows() {
    let mut store = knot_agents::AgentStore::new();
    let ws = workspace("One");
    store.add_workspace(ws.clone());
    store.set_current_workspace(ws.id);
    let id = store.create("~/alpha", knot_agents::CreateOptions::default());
    let unread_counts = BTreeMap::from([(id, 3)]);

    let model = layout_model(&store, None, &[], &unread_counts);

    assert_eq!(model.selected_agent_rows[0].unread_count, 3);
}

#[test]
fn terminal_status_updates_the_shared_agent_store() {
    let mut store = knot_agents::AgentStore::new();
    let id = store.create("~/alpha", knot_agents::CreateOptions::default());
    let shared = Arc::new(Mutex::new(store));

    apply_terminal_status(&shared, id, knot_agents::AgentState::Running);

    assert_eq!(shared.lock().unwrap().agent(id).unwrap().state,
               knot_agents::AgentState::Running);
}

/// An idle, MCP-enabled, non-shell agent with a live session and an unread
/// message it has not been told about is the only case that nudges.
#[test]
fn inbox_prompt_requires_new_unread_message_for_non_shell_mcp_agent() {
    let message = Uuid::new_v4();
    let ready = NudgeCheck { agent_type:     "claude",
                             mcp_enabled:    true,
                             latest_message: Some(message),
                             last_nudged:    None,
                             idle:           true,
                             can_receive:    true, };

    assert!(should_inject_inbox_prompt(ready));
    assert!(!should_inject_inbox_prompt(NudgeCheck { last_nudged: Some(message),
                                                     ..ready }),
            "the same message must not nudge twice");
    assert!(!should_inject_inbox_prompt(NudgeCheck { latest_message: None,
                                                     ..ready }));
    assert!(!should_inject_inbox_prompt(NudgeCheck { agent_type: "shell",
                                                     ..ready }),
            "shell agents cannot receive messages");
    assert!(!should_inject_inbox_prompt(NudgeCheck { mcp_enabled: false,
                                                     ..ready }));
    assert!(!should_inject_inbox_prompt(NudgeCheck { idle: false,
                                                     ..ready }),
            "a working agent is nudged when it next goes idle, not now");
    assert!(!should_inject_inbox_prompt(NudgeCheck { can_receive: false,
                                                     ..ready }),
            "no live session able to take a prompt means no nudge yet");
}

/// A later message re-nudges: the rule is once per message, not once per
/// agent.
#[test]
fn a_second_message_nudges_again() {
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    let check = NudgeCheck { agent_type:     "claude",
                             mcp_enabled:    true,
                             latest_message: Some(second),
                             last_nudged:    Some(first),
                             idle:           true,
                             can_receive:    true, };
    assert!(should_inject_inbox_prompt(check));
}

#[test]
fn awaiting_notice_skips_active_empty_and_duplicate_messages() {
    let agent = Uuid::new_v4();
    assert!(should_show_awaiting_notice(None, agent, "Question?", None));
    assert!(!should_show_awaiting_notice(Some(agent), agent, "Question?", None));
    assert!(!should_show_awaiting_notice(None, agent, "", None));
    assert!(!should_show_awaiting_notice(None, agent, "Question?", Some(&"Question?".to_string())));
}

#[test]
fn agent_status_snapshot_tracks_roster_state_and_registration() {
    let mut store = knot_agents::AgentStore::new();
    let ws = workspace("One");
    store.add_workspace(ws.clone());
    store.set_current_workspace(ws.id);
    let id = store.create("~/alpha", knot_agents::CreateOptions::default());
    store.create("~/beta", knot_agents::CreateOptions::default());

    let snapshot = agent_status_snapshot(&store);
    assert_eq!(snapshot.len(), 2);
    assert!(snapshot.iter()
                    .find(|key| key.id == id)
                    .unwrap()
                    .status_text
                    .is_empty());

    store.set_state(id, knot_agents::AgentState::Running);
    store.set_status_text(id, "planning".to_string());
    store.set_registered(id, true);
    let updated = agent_status_snapshot(&store);
    assert_ne!(snapshot, updated);
    let key = updated.iter().find(|key| key.id == id).unwrap();
    assert_eq!(key.state, knot_agents::AgentState::Running);
    assert_eq!(key.status_text, "planning");
    assert!(key.is_registered);
    assert_eq!(updated.iter().find(|key| key.id != id).unwrap().state,
               knot_agents::AgentState::Idle);
}

#[test]
fn build_agent_store_restores_layout_when_enabled() {
    let agent_id = Uuid::new_v4();
    let saved = knot_core::SavedAgent::new(agent_id, "alpha", None, "~/alpha");
    let mut ws = workspace("Restored");
    ws.agent_ids = vec![agent_id];

    let mut settings = knot_core::Settings::default();
    settings.restore_layout_on_launch = true;
    settings.saved_agents = vec![saved];
    settings.saved_workspaces = vec![ws.clone()];

    let store = build_agent_store(&settings);
    assert_eq!(store.agents().len(), 1);
    assert_eq!(store.workspaces(), &[ws.clone()]);
    assert_eq!(store.current_workspace_id(), Some(ws.id));
}

#[test]
fn build_agent_store_restores_exact_session_id_when_conversation_enabled() {
    let agent_id = Uuid::new_v4();
    let mut saved = knot_core::SavedAgent::new(agent_id, "alpha", None, "~/alpha");
    saved.session_id = Some("s7".to_string());

    let mut settings = knot_core::Settings::default();
    settings.restore_layout_on_launch = true;
    settings.restore_conversation_on_launch = true;
    settings.saved_agents = vec![saved];

    let store = build_agent_store(&settings);
    let agent = store.agent(agent_id).unwrap();
    assert_eq!(agent.resume_session_id.as_deref(), Some("s7"));
    assert!(agent.session_id.is_none());
}

#[test]
fn build_agent_store_leaves_resume_session_unset_when_conversation_disabled() {
    let agent_id = Uuid::new_v4();
    let mut saved = knot_core::SavedAgent::new(agent_id, "alpha", None, "~/alpha");
    saved.session_id = Some("s7".to_string());

    let mut settings = knot_core::Settings::default();
    settings.restore_layout_on_launch = true;
    settings.restore_conversation_on_launch = false;
    settings.saved_agents = vec![saved];

    let store = build_agent_store(&settings);
    let agent = store.agent(agent_id).unwrap();
    assert!(agent.resume_session_id.is_none());
}

#[test]
fn build_agent_store_starts_empty_when_restore_disabled() {
    let mut settings = knot_core::Settings::default();
    settings.restore_layout_on_launch = false;
    settings.saved_agents =
        vec![knot_core::SavedAgent::new(Uuid::new_v4(), "alpha", None, "~/alpha")];

    let store = build_agent_store(&settings);
    assert!(store.agents().is_empty());
    assert!(store.workspaces().is_empty());
}

#[test]
fn initial_selection_prefers_active_agent_and_skips_stale_ids() {
    let mut store = knot_agents::AgentStore::new();
    let ws = workspace("One");
    store.add_workspace(ws.clone());
    store.set_current_workspace(ws.id);
    let first = store.create("~/first", knot_agents::CreateOptions::default());
    let second = store.create("~/second", knot_agents::CreateOptions::default());

    let mut saved = store.saved_workspaces()[0].clone();
    saved.active_agent_ids = vec![Uuid::new_v4(), second];
    let restored = knot_agents::AgentStore::from_saved(&store.saved_agents(false), vec![saved]);

    assert_eq!(initial_agent_selection(&restored), Some(second));
    assert_ne!(initial_agent_selection(&restored), Some(first));
}

#[test]
fn should_notify_requires_setting_and_notice() {
    assert!(should_notify(true, true));
    assert!(!should_notify(false, true));
    assert!(!should_notify(true, false));
    assert!(!should_notify(false, false));
}

#[test]
fn notification_body_uses_message_when_present() {
    assert_eq!(notification_body("Grant access?"), "Grant access?");
}

#[test]
fn notification_body_defaults_on_empty() {
    assert_eq!(notification_body(""), AWAITING_INPUT_DEFAULT_BODY);
}

#[test]
fn notification_response_agent_id_parses_valid_tag() {
    let id = Uuid::new_v4();
    let response = SystemNotificationResponse { tag:       id.to_string().into(),
                                                action_id: None, };
    assert_eq!(notification_response_agent_id(&response), Some(id));
}

#[test]
fn notification_response_agent_id_none_for_invalid_tag() {
    let response = SystemNotificationResponse { tag:       "not-a-uuid".into(),
                                                action_id: None, };
    assert_eq!(notification_response_agent_id(&response), None);
}

#[test]
fn appearance_label_maps_known_modes() {
    assert_eq!(SettingsWindow::appearance_label("system"), "System");
    assert_eq!(SettingsWindow::appearance_label("light"), "Light");
    assert_eq!(SettingsWindow::appearance_label("dark"), "Dark");
}

#[test]
fn appearance_label_defaults_to_auto() {
    assert_eq!(SettingsWindow::appearance_label("auto"), "Auto");
    assert_eq!(SettingsWindow::appearance_label("anything-else"), "Auto");
}

#[test]
fn agent_type_label_maps_known_types() {
    assert_eq!(SettingsWindow::agent_type_label("codex"), "Codex");
    assert_eq!(SettingsWindow::agent_type_label("opencode"), "OpenCode");
    assert_eq!(SettingsWindow::agent_type_label("gemini"), "Gemini");
    assert_eq!(SettingsWindow::agent_type_label("copilot"), "Copilot");
    assert_eq!(SettingsWindow::agent_type_label("custom1"), "Custom 1");
    assert_eq!(SettingsWindow::agent_type_label("custom2"), "Custom 2");
    assert_eq!(SettingsWindow::agent_type_label("shell"), "Shell");
}

#[test]
fn agent_type_label_defaults_to_claude() {
    assert_eq!(SettingsWindow::agent_type_label("claude"), "Claude");
    assert_eq!(SettingsWindow::agent_type_label("anything-else"), "Claude");
}

#[test]
fn persona_preview_returns_short_instructions_unchanged() {
    assert_eq!(SettingsWindow::persona_preview("be terse", 80), "be terse");
}

/// A persona assigned to an agent can't be deleted - the count drives
/// both the disabled delete button and its tooltip.
#[test]
fn personas_in_use_counts_only_the_agents_that_reference_each_persona() {
    let assigned = Uuid::new_v4();
    let unused = Uuid::new_v4();
    let mut store = knot_agents::AgentStore::new();
    let ws = workspace("One");
    store.add_workspace(ws.clone());
    store.set_current_workspace(ws.id);
    store.create("~/alpha",
                 knot_agents::CreateOptions { persona_id: Some(assigned),
                                              ..Default::default() });
    store.create("~/beta",
                 knot_agents::CreateOptions { persona_id: Some(assigned),
                                              ..Default::default() });
    store.create("~/gamma", knot_agents::CreateOptions::default());

    let in_use = SettingsWindow::personas_in_use(store.agents());

    assert_eq!(in_use.get(&assigned).copied(), Some(2));
    assert_eq!(in_use.get(&unused).copied(), None);
}

#[test]
fn persona_delete_tooltip_names_the_reason_it_is_disabled() {
    assert_eq!(SettingsWindow::persona_delete_tooltip(0), "Delete persona");
    assert_eq!(SettingsWindow::persona_delete_tooltip(1),
               "In use by 1 agent");
    assert_eq!(SettingsWindow::persona_delete_tooltip(3),
               "In use by 3 agents");
}

#[test]
fn persona_preview_truncates_long_instructions_with_ellipsis() {
    let instructions = "a".repeat(100);
    let preview = SettingsWindow::persona_preview(&instructions, 80);
    assert_eq!(preview.chars().count(), 81);
    assert!(preview.ends_with('…'));
    assert_eq!(&preview[..80], "a".repeat(80).as_str());
}

#[test]
fn ai_provider_label_maps_known_providers() {
    assert_eq!(SettingsWindow::ai_provider_label("openai"), "OpenAI");
    assert_eq!(SettingsWindow::ai_provider_label("anthropic"), "Anthropic");
    assert_eq!(SettingsWindow::ai_provider_label("google"), "Google");
}

#[test]
fn ai_provider_label_defaults_to_openai() {
    assert_eq!(SettingsWindow::ai_provider_label("anything-else"), "OpenAI");
}

#[test]
fn ai_model_for_matches_swift_reference_defaults() {
    assert_eq!(SettingsWindow::ai_model_for("openai"), "gpt-5-mini");
    assert_eq!(SettingsWindow::ai_model_for("anthropic"),
               "claude-haiku-4-5");
    assert_eq!(SettingsWindow::ai_model_for("google"),
               "gemini-flash-lite-latest");
    assert_eq!(SettingsWindow::ai_model_for("anything-else"), "");
}

#[test]
fn autopilot_action_label_maps_known_actions() {
    assert_eq!(SettingsWindow::autopilot_action_label("mark"),
               "Mark conversation");
    assert_eq!(SettingsWindow::autopilot_action_label("ask"), "Ask me");
    assert_eq!(SettingsWindow::autopilot_action_label("continue"),
               "Auto-continue");
    assert_eq!(SettingsWindow::autopilot_action_label("custom"), "Custom");
}

#[test]
fn autopilot_action_label_defaults_to_mark() {
    assert_eq!(SettingsWindow::autopilot_action_label("anything-else"),
               "Mark conversation");
}

#[test]
fn autopilot_action_description_is_distinct_per_action() {
    let descriptions: BTreeSet<&str> = ["mark", "ask", "continue", "custom"]
        .iter()
        .map(|action| SettingsWindow::autopilot_action_description(action))
        .collect();
    assert_eq!(descriptions.len(), 4);
}

#[test]
fn key_name_for_code_maps_known_modifier_codes() {
    assert_eq!(SettingsWindow::key_name_for_code(54), "Right Command");
    assert_eq!(SettingsWindow::key_name_for_code(56), "Left Shift");
    assert_eq!(SettingsWindow::key_name_for_code(63), "Fn");
}

#[test]
fn key_name_for_code_falls_back_for_unknown_codes() {
    assert_eq!(SettingsWindow::key_name_for_code(999), "Key 999");
}

#[test]
fn mcp_server_url_formats_localhost_with_port() {
    assert_eq!(SettingsWindow::mcp_server_url(8767),
               "http://127.0.0.1:8767/mcp");
    assert_eq!(SettingsWindow::mcp_server_url(9000),
               "http://127.0.0.1:9000/mcp");
}

/// The URL the settings window shows, and the `mcp add` command built from
/// it, must be the one Knot itself hands an agent - not a second spelling.
/// The server routes MCP at `/mcp` and answers a POST to `/` with 405, so
/// the old path-less URL registered a server that could never connect.
#[test]
fn the_displayed_mcp_url_is_the_one_knot_gives_its_own_agents() {
    let mut settings = knot_core::Settings::default();
    settings.mcp_server_port = 8767;
    assert_eq!(SettingsWindow::mcp_server_url(settings.mcp_server_port),
               knot_agent_launch::mcp_url(&settings));
}

#[test]
fn mcp_install_command_matches_swift_reference_per_agent() {
    // The real URL, path included: this command is copied verbatim.
    let url = "http://127.0.0.1:8767/mcp";
    assert_eq!(SettingsWindow::mcp_install_command("claude", url),
               "claude mcp add --transport http --scope user knot http://127.0.0.1:8767/mcp");
    assert_eq!(SettingsWindow::mcp_install_command("codex", url),
               "codex mcp add knot --url http://127.0.0.1:8767/mcp");
    assert_eq!(SettingsWindow::mcp_install_command("opencode", url),
               "opencode mcp add");
    assert_eq!(SettingsWindow::mcp_install_command("gemini", url),
               "gemini mcp add --transport http knot http://127.0.0.1:8767/mcp --scope user");
    assert_eq!(SettingsWindow::mcp_install_command("copilot", url), "");
}

#[test]
fn restore_conversation_toggle_enabled_only_with_layout_restore() {
    assert!(SettingsWindow::restore_conversation_toggle_enabled(true));
    assert!(!SettingsWindow::restore_conversation_toggle_enabled(false));
}

#[test]
fn turning_off_layout_restore_does_not_touch_conversation_restore() {
    let mut settings = knot_core::Settings::default();
    settings.restore_conversation_on_launch = true;
    settings.restore_layout_on_launch = false;
    assert!(settings.restore_conversation_on_launch);
}

#[test]
fn settings_tab_default_is_general() {
    assert_eq!(SettingsTab::ALL[0], SettingsTab::General);
}

#[test]
fn settings_tab_labels_are_distinct() {
    let labels: BTreeSet<&str> = SettingsTab::ALL.iter().map(|tab| tab.label()).collect();
    assert_eq!(labels.len(), SettingsTab::ALL.len());
}

#[test]
fn settings_tab_covers_every_swift_pane() {
    let labels: Vec<&str> = SettingsTab::ALL.iter().map(|tab| tab.label()).collect();
    assert_eq!(labels,
               vec!["General",
                    "Coding",
                    "Personas",
                    "Autopilot",
                    "Voice",
                    "MCP",
                    "Appearance"]);
}

fn config_option(id: &str, category: &str) -> knot_acp::ConfigOption {
    knot_acp::ConfigOption { id:            id.to_string(),
                             name:          id.to_string(),
                             category:      Some(category.to_string()),
                             kind:          "select".to_string(),
                             current_value: serde_json::Value::Null,
                             options:       Vec::new(), }
}

#[test]
fn find_config_option_matches_category_case_insensitively() {
    let options = vec![config_option("mode", "Mode"),
                       config_option("model", "model"),];

    let found = WorkspaceWindow::find_config_option(&options, &["mode"]);
    assert_eq!(found.map(|option| option.id.as_str()), Some("mode"));
}

#[test]
fn find_config_option_is_none_when_no_category_matches() {
    let options = vec![config_option("mode", "mode")];

    assert!(WorkspaceWindow::find_config_option(&options, &["model"]).is_none());
}

#[test]
fn find_config_option_ignores_non_select_options() {
    let mut boolean_option = config_option("brave_mode", "mode");
    boolean_option.kind = "boolean".to_string();
    let options = vec![boolean_option];

    assert!(WorkspaceWindow::find_config_option(&options, &["mode"]).is_none());
}

/// Regression guard for "Remove Agent does nothing": `AgentStore::remove`
/// only mutates memory, and nothing writes settings on quit, so unless the
/// caller re-snapshots `saved_agents`/`saved_workspaces` afterwards the
/// removal is undone by the next launch. `WorkspaceWindow::remove_agent`
/// does that snapshot; this pins the round trip it depends on.
#[test]
fn removing_an_agent_survives_a_settings_round_trip() {
    let mut store = knot_agents::AgentStore::new();
    let ws = workspace("One");
    store.add_workspace(ws.clone());
    store.set_current_workspace(ws.id);
    let doomed = store.create("~/doomed", knot_agents::CreateOptions::default());
    let kept = store.create("~/kept", knot_agents::CreateOptions::default());

    assert_eq!(store.remove(doomed).len(), 1);

    let mut settings = knot_core::Settings::default();
    settings.restore_layout_on_launch = true;
    settings.saved_agents = store.saved_agents(false);
    settings.saved_workspaces = store.saved_workspaces();

    let restored = build_agent_store(&settings);
    assert!(restored.agent(doomed).is_none(),
            "a removed agent must not come back after a restore");
    assert!(restored.agent(kept).is_some());
    assert_eq!(restored.workspaces()[0].agent_ids, vec![kept]);
}

/// Removing an agent takes its shell companions with it, so the persisted
/// snapshot must lose them too rather than leaving orphans behind.
#[test]
fn removing_an_agent_also_drops_its_companions_from_the_snapshot() {
    let mut store = knot_agents::AgentStore::new();
    let ws = workspace("One");
    store.add_workspace(ws.clone());
    store.set_current_workspace(ws.id);
    let parent = store.create("~/parent", knot_agents::CreateOptions::default());
    store.create_shell_companion(parent)
         .expect("companion should be creatable");
    assert_eq!(store.agents().len(), 2);

    assert_eq!(store.remove(parent).len(), 2);

    let mut settings = knot_core::Settings::default();
    settings.restore_layout_on_launch = true;
    settings.saved_agents = store.saved_agents(false);
    settings.saved_workspaces = store.saved_workspaces();

    assert!(build_agent_store(&settings).agents().is_empty());
}

/// The queued-row controls carry no visible text of their own, so their
/// tooltips and accessibility labels are the only thing naming them - a
/// missing key would ship the key string itself as the button's name.
#[test]
fn queued_message_control_labels_resolve() {
    for key in ["panel.queued",
                "panel.failed",
                "panel.retry",
                "panel.retry_queued",
                "panel.delete_queued",
                "panel.edit_queued",
                "panel.replace_composer_title",
                "panel.replace_composer_body",
                "panel.retry_connect"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

#[test]
fn the_queued_status_label_follows_the_failed_mark() {
    assert_eq!(workspace_window::prompt_queue::queued_status_label(false),
               knot_core::l10n::t("panel.queued"));
    assert_eq!(workspace_window::prompt_queue::queued_status_label(true),
               knot_core::l10n::t("panel.failed"));
}
