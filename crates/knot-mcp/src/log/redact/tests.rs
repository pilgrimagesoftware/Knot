use serde_json::json;

use super::describe_call;

#[test]
fn the_tool_name_and_argument_keys_are_named() {
    let described = describe_call("send-message",
                                  &json!({ "agentId": "a1", "message": "hello" }));

    assert!(described.starts_with("tools/call send-message"));
    assert!(described.contains("keys=[agentId, message]"));
}

#[test]
fn argument_values_never_appear() {
    let secret = "the user's prompt, verbatim, with a path at /Users/someone/private";
    let described = describe_call("send-message",
                                  &json!({ "agentId": "a1", "message": secret }));

    assert!(!described.contains(secret),
            "a durable file must not hold what the agent was asked: {described}");
    assert!(!described.contains("/Users/someone/private"));
    assert!(!described.contains("a1"), "even a short value is a value");
}

#[test]
fn nested_values_do_not_leak_through_a_nested_object() {
    let described = describe_call("plan-tasks",
                                  &json!({ "tasks": [ { "goal": "ship the thing" } ] }));

    assert!(described.contains("keys=[tasks]"));
    assert!(!described.contains("ship the thing"),
            "only top-level key names are reported, never anything under them");
    assert!(!described.contains("goal"));
}

#[test]
fn the_payload_size_is_reported() {
    let arguments = json!({ "agentId": "a1" });
    let expected = arguments.to_string().len();

    let described = describe_call("set-status", &arguments);

    assert!(described.contains(&format!("bytes={expected}")),
            "size is what is left to say about a payload whose content is withheld: {described}");
}

#[test]
fn empty_arguments_report_no_keys() {
    let described = describe_call("list-agents", &json!({}));

    assert!(described.contains("keys=[]"));
    assert!(described.contains("bytes=2"),
            "`{{}}` is two bytes: {described}");
}

#[test]
fn a_payload_that_is_not_an_object_is_reported_by_size_alone() {
    let described = describe_call("odd", &json!(["first", "second"]));

    assert!(described.contains("keys=<not an object>"));
    assert!(!described.contains("first") && !described.contains("second"),
            "an array's elements are still values: {described}");
}
