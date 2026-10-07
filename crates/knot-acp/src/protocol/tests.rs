//! Parsing the ACP wire shapes into [`super`]'s types.

use super::*;

#[test]
fn text_delta_parses_from_the_real_nested_update_envelope() {
    // Captured against a live `gemini --acp` handshake (task 1.2):
    // the notification's params nest the typed update, and the text
    // itself is a content block, not a flat field.
    let params = serde_json::json!({
        "sessionId": "s1",
        "update": {
            "sessionUpdate": "agent_message_chunk",
            "content": { "type": "text", "text": "pong" }
        }
    });

    let update = SessionUpdate::from_params(params);

    assert!(matches!(update, SessionUpdate::TextDelta { text } if text == "pong"));
}

/// What `session/load` replays the user's side of a conversation as. It
/// used to fall through to `Unknown`, which left a restored
/// conversation with the agent's replies and none of the prompts.
#[test]
fn user_message_chunk_parses_as_the_users_text() {
    let params = serde_json::json!({
        "sessionId": "s1",
        "update": {
            "sessionUpdate": "user_message_chunk",
            "content": { "type": "text", "text": "fix the build" }
        }
    });

    let update = SessionUpdate::from_params(params);

    assert!(matches!(update,
                     SessionUpdate::UserMessageChunk { text, meta: None } if text == "fix the build"));
}

/// The chunk's `_meta` is kept whole, so an origin tag an adapter adds
/// reaches whoever decides how to show the chunk.
#[test]
fn user_message_chunk_keeps_its_meta() {
    let params = serde_json::json!({
        "sessionId": "s1",
        "update": {
            "sessionUpdate": "user_message_chunk",
            "content": { "type": "text", "text": "<task-notification/>" },
            "_meta": { "_claude/origin": { "kind": "task-notification" } }
        }
    });

    let SessionUpdate::UserMessageChunk { meta, .. } = SessionUpdate::from_params(params)
    else {
        panic!("a user message chunk");
    };

    assert_eq!(meta.as_ref()
                   .and_then(|meta| meta.pointer("/_claude~1origin/kind"))
                   .and_then(Value::as_str),
               Some("task-notification"));
}

/// Wire shape taken verbatim from the ACP tool-call docs
/// (<https://agentclientprotocol.com/protocol/tool-calls> -
/// "Creating").
#[test]
fn tool_call_start_carries_title_status_and_content() {
    let params = serde_json::json!({
        "sessionId": "s1",
        "update": {
            "sessionUpdate": "tool_call",
            "toolCallId": "call_001",
            "title": "Reading configuration file",
            "kind": "read",
            "status": "pending"
        }
    });

    let SessionUpdate::ToolCallStart { tool_call_id,
                                       kind,
                                       title,
                                       status,
                                       content,
                                       raw_input,
                                       meta, } = SessionUpdate::from_params(params)
    else {
        panic!("expected a tool-call start");
    };
    assert_eq!(tool_call_id, "call_001");
    assert_eq!(kind, "read");
    assert_eq!(title, "Reading configuration file");
    assert_eq!(status, "pending");
    assert!(content.is_empty());
    // An agent that sent neither is distinguishable from one that sent an
    // empty object - see `raw_input_and_meta_are_absent_not_empty`.
    assert_eq!(raw_input, None);
    assert_eq!(meta, None);
}

/// The two fields adapters use to say what a tool actually is, since ACP's
/// own `kind` is an icon hint and `title` is prose. Carried verbatim,
/// vendor keys and all.
#[test]
fn a_tool_call_start_carries_raw_input_and_meta() {
    let params = serde_json::json!({
        "sessionId": "sess_1",
        "update": {
            "sessionUpdate": "tool_call",
            "toolCallId": "call_001",
            "title": "Map the callers",
            "kind": "think",
            "status": "pending",
            "rawInput": { "subagent_type": "discovery", "description": "Map the callers" },
            "_meta": { "claudeCode": { "toolName": "Task", "subagent": true } }
        }
    });

    let SessionUpdate::ToolCallStart { raw_input, meta, .. } = SessionUpdate::from_params(params)
    else {
        panic!("expected a tool-call start");
    };

    assert_eq!(raw_input.as_ref()
                        .and_then(|input| input.get("subagent_type")),
               Some(&serde_json::json!("discovery")));
    assert_eq!(meta.as_ref()
                   .and_then(|meta| meta.pointer("/claudeCode/subagent")),
               Some(&serde_json::json!(true)));
}

/// Absent is not empty. A consumer treats `None` as "the agent said
/// nothing", and an empty object as "the agent said there is nothing" -
/// only the second is a statement about the call.
#[test]
fn raw_input_and_meta_are_absent_not_empty() {
    let params = serde_json::json!({
        "sessionId": "sess_1",
        "update": {
            "sessionUpdate": "tool_call",
            "toolCallId": "call_001",
            "title": "t",
            "kind": "read",
            "status": "pending",
            "rawInput": {},
            "_meta": {}
        }
    });

    let SessionUpdate::ToolCallStart { raw_input, meta, .. } = SessionUpdate::from_params(params)
    else {
        panic!("expected a tool-call start");
    };

    assert_eq!(raw_input, Some(serde_json::json!({})));
    assert_eq!(meta, Some(serde_json::json!({})));
}

/// A partial update carries neither, and the spec's "only the fields being
/// changed need to be included" makes that "no change", not "cleared".
#[test]
fn a_status_only_update_carries_neither_field() {
    let params = serde_json::json!({
        "sessionId": "sess_1",
        "update": {
            "sessionUpdate": "tool_call_update",
            "toolCallId": "call_001",
            "status": "completed"
        }
    });

    let SessionUpdate::ToolCallUpdate { status,
                                        raw_input,
                                        meta,
                                        .. } = SessionUpdate::from_params(params)
    else {
        panic!("expected a tool-call update");
    };

    assert_eq!(status.as_deref(), Some("completed"));
    assert_eq!(raw_input, None);
    assert_eq!(meta, None);
}

/// An omitted `kind`/`status` takes the spec's documented defaults
/// (`other`/`pending`) rather than an empty string.
#[test]
fn tool_call_start_defaults_kind_and_status() {
    let params = serde_json::json!({
        "update": { "sessionUpdate": "tool_call", "toolCallId": "c1" }
    });

    let SessionUpdate::ToolCallStart { kind, status, .. } = SessionUpdate::from_params(params)
    else {
        panic!("expected a tool-call start");
    };
    assert_eq!(kind, "other");
    assert_eq!(status, "pending");
}

/// The real reason tool-call cards used to hang on "Running…": a
/// call's output arrives as an update's `content` array, not as a
/// `tool_call_result` session update (no such kind exists).
#[test]
fn tool_call_update_carries_its_output_as_content() {
    let params = serde_json::json!({
        "sessionId": "s1",
        "update": {
            "sessionUpdate": "tool_call_update",
            "toolCallId": "call_001",
            "status": "completed",
            "content": [
                {
                    "type": "content",
                    "content": { "type": "text", "text": "Found 3 configuration files..." }
                }
            ]
        }
    });

    let SessionUpdate::ToolCallUpdate { tool_call_id,
                                        status,
                                        content,
                                        .. } = SessionUpdate::from_params(params)
    else {
        panic!("expected a tool-call update");
    };
    assert_eq!(tool_call_id, "call_001");
    assert_eq!(status.as_deref(), Some("completed"));
    assert_eq!(content,
               vec![ToolCallContent::Text("Found 3 configuration files...".to_string())]);
}

/// An update that changes only `content` must report `status: None`
/// so the caller keeps the status it already knows, per the spec's
/// "only the fields being changed need to be included".
#[test]
fn tool_call_update_omitting_status_reports_it_as_absent() {
    let params = serde_json::json!({
        "update": { "sessionUpdate": "tool_call_update", "toolCallId": "c1" }
    });

    let SessionUpdate::ToolCallUpdate { status, title, .. } = SessionUpdate::from_params(params)
    else {
        panic!("expected a tool-call update");
    };
    assert_eq!(status, None);
    assert_eq!(title, None);
}

#[test]
fn tool_call_content_parses_diffs_terminals_and_skips_unknown_types() {
    let params = serde_json::json!({
        "update": {
            "sessionUpdate": "tool_call_update",
            "toolCallId": "c1",
            "content": [
                {
                    "type": "diff",
                    "path": "/home/user/project/src/config.json",
                    "oldText": "{\n  \"debug\": false\n}",
                    "newText": "{\n  \"debug\": true\n}"
                },
                { "type": "terminal", "terminalId": "term_xyz789" },
                { "type": "some-future-block" }
            ]
        }
    });

    let SessionUpdate::ToolCallUpdate { content, .. } = SessionUpdate::from_params(params)
    else {
        panic!("expected a tool-call update");
    };
    assert_eq!(content,
               vec![ToolCallContent::Diff { path:
                                                "/home/user/project/src/config.json".to_string(),
                                            old_text: Some("{\n  \"debug\": false\n}".to_string()),
                                            new_text: "{\n  \"debug\": true\n}".to_string(), },
                    ToolCallContent::Terminal { terminal_id: "term_xyz789".to_string(), }]);
}

/// A brand-new file has no `oldText`, which must stay distinguishable
/// from an empty one so the diff renders as all-additions.
#[test]
fn a_new_file_diff_has_no_old_text() {
    let params = serde_json::json!({
        "update": {
            "sessionUpdate": "tool_call_update",
            "toolCallId": "c1",
            "content": [{ "type": "diff", "path": "new.rs", "newText": "fn main() {}" }]
        }
    });

    let SessionUpdate::ToolCallUpdate { content, .. } = SessionUpdate::from_params(params)
    else {
        panic!("expected a tool-call update");
    };
    assert_eq!(content,
               vec![ToolCallContent::Diff { path:     "new.rs".to_string(),
                                            old_text: None,
                                            new_text: "fn main() {}".to_string(), }]);
}

#[test]
fn text_delta_still_parses_the_flat_legacy_shape() {
    let params = serde_json::json!({ "sessionUpdate": "text_delta", "text": "hi" });

    let update = SessionUpdate::from_params(params);

    assert!(matches!(update, SessionUpdate::TextDelta { text } if text == "hi"));
}

#[test]
fn unrecognized_nested_update_kind_is_unknown() {
    let params = serde_json::json!({
        "sessionId": "s1",
        "update": { "sessionUpdate": "available_commands_update", "availableCommands": [] }
    });

    let update = SessionUpdate::from_params(params.clone());

    assert!(matches!(update, SessionUpdate::Unknown { raw } if raw == params));
}

#[test]
fn usage_update_parses_context_window_tokens() {
    let params = serde_json::json!({
        "update": {
            "sessionUpdate": "usage_update",
            "used": 53_000,
            "size": 200_000
        }
    });

    assert!(matches!(SessionUpdate::from_params(params),
                     SessionUpdate::Usage { used: 53_000,
                                            size: 200_000, }));
}

/// The `initialize` response from the spec's own example
/// (<https://agentclientprotocol.com/protocol/initialization>), not a shape
/// inferred from [`AgentInfo`].
fn spec_initialize_result() -> Value {
    serde_json::json!({
        "protocolVersion": 1,
        "agentCapabilities": {
            "loadSession": true,
            "promptCapabilities": { "image": true, "audio": true, "embeddedContext": true },
            "mcpCapabilities": { "http": true, "sse": true }
        },
        "agentInfo": { "name": "my-agent", "title": "My Agent", "version": "1.0.0" },
        "authMethods": []
    })
}

#[test]
fn agent_info_parses_from_the_spec_example() {
    let result: InitializeResult =
        serde_json::from_value(spec_initialize_result()).expect("the spec's example parses");

    let info = result.agent_info.expect("the example carries agentInfo");
    assert_eq!(info.name, "my-agent");
    assert_eq!(info.display_name(), "My Agent");
    assert_eq!(info.version(), Some("1.0.0"));
}

/// `agentInfo` is a SHOULD: an agent that leaves it out still connects.
#[test]
fn an_absent_agent_info_reads_as_none() {
    let mut raw = spec_initialize_result();
    raw.as_object_mut().expect("an object").remove("agentInfo");

    let result: InitializeResult = serde_json::from_value(raw).expect("still parses");

    assert_eq!(result.agent_info, None);
    assert!(result.capabilities.supports_resume,
            "dropping agentInfo must not disturb the fields beside it");
}

/// A field the client only displays must not fail the handshake.
#[test]
fn a_malformed_agent_info_reads_as_none_rather_than_failing_initialize() {
    for malformed in [serde_json::json!({ "title": "No Name" }),
                      serde_json::json!("my-agent 1.0.0"),
                      Value::Null]
    {
        let mut raw = spec_initialize_result();
        raw["agentInfo"] = malformed.clone();

        let result: InitializeResult =
            serde_json::from_value(raw).unwrap_or_else(|error| {
                                           panic!("{malformed} failed initialize: {error}")
                                       });

        assert_eq!(result.agent_info, None, "{malformed}");
    }
}

#[test]
fn display_name_falls_back_to_name_and_blank_fields_count_as_absent() {
    let info: AgentInfo =
        serde_json::from_value(serde_json::json!({ "name": "codex-acp", "title": "  ",
                                                   "version": "" })).expect("parses");

    assert_eq!(info.display_name(), "codex-acp");
    assert_eq!(info.version(), None);
}
