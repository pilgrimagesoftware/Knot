use std::path::Path;

use knot_core::{PersonaState, PersonaType};

use super::*;

fn id() -> Uuid {
    Uuid::nil()
}

fn persona(instructions: &str) -> Persona {
    Persona { id:           Uuid::nil(),
              name:         "Ada".to_string(),
              instructions: instructions.to_string(),
              persona_type: PersonaType::User,
              state:        PersonaState::Enabled, }
}

fn nothing_inherited(_: &str) -> Option<String> {
    None
}

fn deliver(carrier: InstructionCarrier) -> InstructionDelivery {
    instruction_delivery(DeliveryRequest { carrier,
                                           agent_id: id(),
                                           instructions: "KNOT".to_owned(),
                                           cache_dir: Some(Path::new("/cache")),
                                           inherited: &nothing_inherited })
}

fn env_json(delivery: &InstructionDelivery, name: &str) -> Value {
    let (_, value) = delivery.env
                             .iter()
                             .find(|(key, _)| key == name)
                             .unwrap_or_else(|| panic!("{name} is not set"));
    serde_json::from_str(value).expect("the variable holds JSON")
}

#[test]
fn each_confirmed_adapter_has_its_own_carrier() {
    assert_eq!(instruction_carrier("claude"),
               InstructionCarrier::SessionMeta);
    assert_eq!(instruction_carrier("codex"),
               InstructionCarrier::CodexConfigEnv);
    assert_eq!(instruction_carrier("opencode"),
               InstructionCarrier::OpencodeConfigEnv);
}

/// An adapter nobody has checked for a system channel keeps what worked
/// before: the instructions ahead of the registration request.
#[test]
fn an_unconfirmed_adapter_falls_back_to_the_first_turn() {
    for agent_type in ["gemini", "copilot", "unknown-type"] {
        assert_eq!(instruction_carrier(agent_type),
                   InstructionCarrier::FirstTurn,
                   "{agent_type}");
    }
}

#[test]
fn standing_instructions_carry_the_knot_instructions_and_the_persona() {
    let p = persona("Be terse.");
    let text = standing_instructions(id(), Some(&p));
    assert!(text.starts_with(&knot_instructions(id())));
    assert!(text.contains("impersonate Ada"));
    assert!(text.contains("Be terse."));
    assert!(!text.contains('\n'),
            "the first-turn fallback must stay one line");
    assert_eq!(standing_instructions(id(), None), knot_instructions(id()));
}

/// A string `systemPrompt` would replace Claude Code's own preset; the
/// object form appends to it.
#[test]
fn claude_gets_an_appended_system_prompt() {
    let delivery = deliver(InstructionCarrier::SessionMeta);
    assert_eq!(delivery.session_meta,
               Some(json!({ "systemPrompt": { "append": "KNOT" } })));
    assert!(delivery.env.is_empty());
    assert_eq!(delivery.first_turn, None);
}

#[test]
fn codex_gets_developer_instructions_in_its_config_env() {
    let delivery = deliver(InstructionCarrier::CodexConfigEnv);
    assert_eq!(env_json(&delivery, "CODEX_CONFIG"),
               json!({ "developer_instructions": "KNOT" }));
    assert_eq!(delivery.session_meta, None);
    assert_eq!(delivery.first_turn, None);
}

/// Knot's environment is the adapter's too, so a `CODEX_CONFIG` the user
/// set is kept, and developer instructions of their own come first.
#[test]
fn codex_config_merges_into_what_the_user_already_set() {
    let inherited = |name: &str| {
        (name == "CODEX_CONFIG").then(|| {
                                    r#"{"model":"o3","developer_instructions":"Mine."}"#.to_owned()
                                })
    };
    let delivery = instruction_delivery(DeliveryRequest { carrier:
                                                              InstructionCarrier::CodexConfigEnv,
                                                          agent_id:     id(),
                                                          instructions: "KNOT".to_owned(),
                                                          cache_dir:    None,
                                                          inherited:    &inherited, });
    assert_eq!(env_json(&delivery, "CODEX_CONFIG"),
               json!({ "model": "o3", "developer_instructions": "Mine.\n\nKNOT" }));
}

#[test]
fn opencode_gets_a_per_agent_file_named_in_its_config_env() {
    let delivery = deliver(InstructionCarrier::OpencodeConfigEnv);
    let path = Path::new("/cache/agent-instructions").join(format!("{}.md", id()));
    assert_eq!(delivery.file,
               Some(InstructionsFile { path:     path.clone(),
                                       contents: "KNOT".to_owned(), }));
    assert_eq!(env_json(&delivery, "OPENCODE_CONFIG_CONTENT"),
               json!({ "instructions": [path.to_string_lossy()] }));
    assert_eq!(delivery.first_turn, None);
}

#[test]
fn opencode_instructions_the_user_set_are_kept() {
    let inherited = |name: &str| {
        (name == "OPENCODE_CONFIG_CONTENT").then(|| r#"{"instructions":["/mine.md"]}"#.to_owned())
    };
    let delivery = instruction_delivery(DeliveryRequest { carrier:
                                                              InstructionCarrier::OpencodeConfigEnv,
                                                          agent_id:     id(),
                                                          instructions: "KNOT".to_owned(),
                                                          cache_dir:    Some(Path::new("/cache")),
                                                          inherited:    &inherited, });
    let files = env_json(&delivery, "OPENCODE_CONFIG_CONTENT")["instructions"].clone();
    assert_eq!(files[0], json!("/mine.md"));
    assert_eq!(files.as_array().map(Vec::len), Some(2));
}

/// With nowhere to put the file, the instructions still arrive - on the
/// first turn, as they did before there was a system channel.
#[test]
fn opencode_without_a_cache_dir_falls_back_to_the_first_turn() {
    let delivery = instruction_delivery(DeliveryRequest { carrier:
                                                              InstructionCarrier::OpencodeConfigEnv,
                                                          agent_id:     id(),
                                                          instructions: "KNOT".to_owned(),
                                                          cache_dir:    None,
                                                          inherited:    &nothing_inherited, });
    assert_eq!(delivery,
               InstructionDelivery { first_turn: Some("KNOT".to_owned()),
                                     ..InstructionDelivery::default() });
}

/// An adapter pointed at a file that was never written would start with no
/// instructions at all, and nothing would say so.
#[test]
fn an_unwritten_file_moves_the_instructions_to_the_first_turn() {
    let written = deliver(InstructionCarrier::OpencodeConfigEnv);
    let delivery = written.clone().into_first_turn();
    assert_eq!(delivery.first_turn.as_deref(), Some("KNOT"));
    assert_eq!(delivery.file, None);
    // The variable also carries the user's options; opencode skips an
    // entry that matches no file.
    assert_eq!(delivery.env, written.env);
    let claude = deliver(InstructionCarrier::SessionMeta);
    assert_eq!(claude.clone().into_first_turn(),
               claude,
               "no file, nothing to move");
}

#[test]
fn the_file_is_written_with_its_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = InstructionsFile { path:     dir.path().join("nested/agent.md"),
                                  contents: "KNOT".to_owned(), };
    file.write().expect("write");
    assert_eq!(std::fs::read_to_string(&file.path).expect("read"), "KNOT");
}

#[test]
fn session_meta_keeps_both_the_options_and_the_instructions() {
    let options = json!({ "claudeCode": { "options": { "extraArgs": { "verbose": null } } } });
    let instructions = json!({ "systemPrompt": { "append": "KNOT" } });
    assert_eq!(merge_session_meta(Some(options.clone()), Some(instructions.clone())),
               Some(json!({ "claudeCode": options["claudeCode"],
                            "systemPrompt": instructions["systemPrompt"] })));
    assert_eq!(merge_session_meta(Some(options.clone()), None),
               Some(options));
    assert_eq!(merge_session_meta(None, Some(instructions.clone())),
               Some(instructions));
    assert_eq!(merge_session_meta(None, None), None);
}

/// Nowhere to write the file must not cost the user their options: the
/// variable is still set, with what it was layered from.
#[test]
fn opencode_without_a_cache_dir_keeps_the_options_in_its_env() {
    let inherited =
        |name: &str| (name == "OPENCODE_CONFIG_CONTENT").then(|| r#"{"model":"a/b"}"#.to_owned());
    let delivery = instruction_delivery(DeliveryRequest { carrier:
                                                              InstructionCarrier::OpencodeConfigEnv,
                                                          agent_id:     id(),
                                                          instructions: "KNOT".to_owned(),
                                                          cache_dir:    None,
                                                          inherited:    &inherited, });
    assert_eq!(env_json(&delivery, "OPENCODE_CONFIG_CONTENT"),
               json!({ "model": "a/b" }));
    assert_eq!(delivery.first_turn.as_deref(), Some("KNOT"));
}

/// The user's options are layered in first and the instructions after, so
/// a `-c developer_instructions=...` of their own keeps Knot's as well.
#[test]
fn an_option_cannot_displace_the_instructions() {
    let options = crate::adapter_options("codex", "-c developer_instructions=Mine -m o3");
    let inherited = |name: &str| options.layered_env(name, None);
    let delivery = instruction_delivery(DeliveryRequest { carrier:
                                                              InstructionCarrier::CodexConfigEnv,
                                                          agent_id:     id(),
                                                          instructions: "KNOT".to_owned(),
                                                          cache_dir:    None,
                                                          inherited:    &inherited, });
    assert_eq!(env_json(&delivery, "CODEX_CONFIG"),
               json!({ "model": "o3", "developer_instructions": "Mine\n\nKNOT" }));
}

/// `-c developer_instructions=42` parses as a number; it is still the
/// user's text, so it is kept ahead of Knot's rather than replaced.
#[test]
fn non_string_developer_instructions_are_kept_as_text() {
    let inherited = |name: &str| {
        (name == "CODEX_CONFIG").then(|| r#"{"developer_instructions":42}"#.to_owned())
    };
    let delivery = instruction_delivery(DeliveryRequest { carrier:
                                                              InstructionCarrier::CodexConfigEnv,
                                                          agent_id:     id(),
                                                          instructions: "KNOT".to_owned(),
                                                          cache_dir:    None,
                                                          inherited:    &inherited, });
    assert_eq!(env_json(&delivery, "CODEX_CONFIG"),
               json!({ "developer_instructions": "42\n\nKNOT" }));
}
