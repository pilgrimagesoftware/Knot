//! Unit tests for [`super`].

use serde_json::json;

use super::*;

fn options(value: serde_json::Value) -> Vec<PermissionOption> {
    serde_json::from_value(value).expect("the options deserialize")
}

/// What Claude's ACP adapter sends for a tool call: Always Allow first.
fn claude() -> Vec<PermissionOption> {
    options(json!([
                { "kind": "allow_always", "name": "Always Allow", "optionId": "allow_always" },
                { "kind": "allow_once", "name": "Allow", "optionId": "allow" },
                { "kind": "reject_once", "name": "Reject", "optionId": "reject" }
            ]))
}

/// What it sends to leave plan mode, where the first option also changes
/// the session's mode.
fn exit_plan_mode() -> Vec<PermissionOption> {
    options(json!([
                { "kind": "allow_always", "name": "Yes, and auto-accept edits", "optionId": "acceptEdits" },
                { "kind": "allow_once", "name": "Yes, and manually approve edits", "optionId": "default" },
                { "kind": "reject_once", "name": "No, keep planning", "optionId": "plan" }
            ]))
}

#[test]
fn kinds_are_parsed() {
    let kinds: Vec<_> = claude().into_iter().map(|option| option.kind).collect();
    assert_eq!(kinds,
               [Some(PermissionOptionKind::AllowAlways),
                Some(PermissionOptionKind::AllowOnce),
                Some(PermissionOptionKind::RejectOnce)]);
}

/// An unknown kind costs that option its kind, not the request its list.
#[test]
fn an_unknown_kind_keeps_the_option() {
    let parsed = options(json!([
                             { "kind": "allow_sometimes", "name": "Maybe", "optionId": "maybe" },
                             { "kind": 7, "name": "Odd", "optionId": "odd" },
                             { "name": "Plain", "optionId": "plain" }
                         ]));
    assert_eq!(parsed.len(), 3);
    assert!(parsed.iter().all(|option| option.kind.is_none()));
}

#[test]
fn kinds_round_trip_through_their_spelling() {
    for kind in PermissionOptionKind::ALL {
        assert_eq!(kind.to_string().parse(), Ok(kind));
    }
    assert!("allow".parse::<PermissionOptionKind>().is_err());
}

/// The bug behind #530: Allow is the allow-once option, wherever it sits.
#[test]
fn allow_answers_once_even_when_always_is_listed_first() {
    assert_eq!(PermissionDecision::Allow.option_id(&claude()).as_deref(),
               Some("allow"));
    assert_eq!(PermissionDecision::AllowAlways.option_id(&claude())
                                              .as_deref(),
               Some("allow_always"));
    assert_eq!(PermissionDecision::Deny.option_id(&claude()).as_deref(),
               Some("reject"));
}

/// Leaving plan mode: a plain Allow must not switch the session to
/// auto-accepting edits.
#[test]
fn allow_does_not_pick_the_mode_changing_option() {
    assert_eq!(PermissionDecision::Allow.option_id(&exit_plan_mode())
                                        .as_deref(),
               Some("default"));
    assert_eq!(PermissionDecision::Deny.option_id(&exit_plan_mode())
                                       .as_deref(),
               Some("plan"));
}

/// An Allow that the request does not offer picks nothing, rather than the
/// always option beside it.
#[test]
fn a_decision_not_on_offer_picks_nothing() {
    let only_always = options(json!([
                                  { "kind": "allow_always", "name": "Always Allow", "optionId": "always" },
                                  { "kind": "reject_once", "name": "Reject", "optionId": "reject" }
                              ]));
    assert_eq!(PermissionDecision::Allow.option_id(&only_always), None);

    let request = PermissionRequest { rpc_id:          json!(1),
                                      tool_call_id:    "tc".to_owned(),
                                      tool_call_title: None,
                                      options:         only_always, };
    assert!(!request.offers(PermissionDecision::Allow));
    assert!(request.offers(PermissionDecision::AllowAlways));
}

#[test]
fn deny_falls_back_to_reject_always() {
    let only_always_reject = options(json!([
                                         { "kind": "allow_once", "name": "Allow", "optionId": "allow" },
                                         { "kind": "reject_always", "name": "Never", "optionId": "never" }
                                     ]));
    assert_eq!(PermissionDecision::Deny.option_id(&only_always_reject)
                                       .as_deref(),
               Some("never"));
}

/// An adapter without kinds is answered as before.
#[test]
fn without_kinds_the_old_reading_applies() {
    let legacy = options(json!([
                             { "optionId": "allow-once", "name": "Allow" },
                             { "optionId": "deny", "name": "Deny" }
                         ]));
    assert_eq!(PermissionDecision::Allow.option_id(&legacy).as_deref(),
               Some("allow-once"));
    assert_eq!(PermissionDecision::Deny.option_id(&legacy).as_deref(),
               Some("deny"));
    assert_eq!(PermissionDecision::AllowAlways.option_id(&legacy), None);
    assert_eq!(PermissionDecision::Allow.option_id(&[]).as_deref(),
               Some("allow"));
    assert_eq!(PermissionDecision::Deny.option_id(&[]).as_deref(),
               Some("deny"));
}

#[test]
fn choosing_an_option_answers_with_it() {
    assert_eq!(PermissionDecision::Choose(0).option_id(&exit_plan_mode())
                                            .as_deref(),
               Some("acceptEdits"));
    assert_eq!(PermissionDecision::Choose(9).option_id(&exit_plan_mode()),
               None);
}
