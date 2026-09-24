//! The ACP status mapping. Covers `activity-detection`'s "ACP updates drive
//! status for Panel-mode agents" and the message source
//! `desktop-notifications` names, at the level that does not need a window: a
//! `PanelState` in, a status and an attention message out.

use knot_acp::PermissionRequest;
use knot_agents::AgentState;

use super::acp_status;
use crate::panel_state::PanelState;

fn permission(title: Option<&str>) -> PermissionRequest {
    PermissionRequest { rpc_id:          serde_json::json!(1),
                        tool_call_id:    "call-1".to_string(),
                        tool_call_title: title.map(str::to_owned),
                        options:         Vec::new(), }
}

#[test]
fn a_pending_permission_is_awaiting_input_and_carries_its_title() {
    let mut state = PanelState::default();
    state.pending_permission = Some(permission(Some("Allow writing to src/main.rs?")));

    assert_eq!(acp_status(&state),
               (AgentState::Input, Some("Allow writing to src/main.rs?".to_string())));
}

#[test]
fn a_pending_permission_wins_over_an_active_turn() {
    // The permission arrives mid-turn, so both flags are set. Awaiting input
    // is the one the user needs to see - `activity-detection`'s "Permission
    // request during a turn".
    let mut state = PanelState::default();
    state.pending_permission = Some(permission(Some("Grant access?")));
    state.turn_active = true;

    assert_eq!(acp_status(&state).0, AgentState::Input);
}

#[test]
fn a_permission_without_a_title_carries_no_message() {
    // The notification falls back to its default body rather than inventing
    // one - `desktop-notifications`' "Notification uses default body without
    // a message".
    let mut state = PanelState::default();
    state.pending_permission = Some(permission(None));

    assert_eq!(acp_status(&state), (AgentState::Input, None));
}

#[test]
fn an_active_turn_is_working() {
    let mut state = PanelState::default();
    state.turn_active = true;

    assert_eq!(acp_status(&state), (AgentState::Running, None));
}

#[test]
fn a_finished_turn_with_nothing_pending_is_idle() {
    // What makes the turn's end a delivery opportunity: Idle is the status
    // that routes through `mark_idle`, which is what emits `CheckMessages`.
    assert_eq!(acp_status(&PanelState::default()), (AgentState::Idle, None));
}

#[test]
fn the_status_is_a_level_that_repeats_unchanged() {
    // The property the caller's "differs from the store" gate depends on:
    // reading the same state twice gives the same answer, so a permission
    // request pending across many ticks is one edge, not many. Without the
    // gate this would be one notification per poll.
    let mut state = PanelState::default();
    state.pending_permission = Some(permission(Some("Grant access?")));

    assert_eq!(acp_status(&state), acp_status(&state));
}

// The level-to-edge gate. What these cover is the tick *sequence*: the
// tracker emits an `AwaitingInput` effect for every `Input` it is handed, so
// a permission request that stays pending must be reported once, not once per
// poll - `desktop-notifications`' "A still-pending permission request is not
// re-notified".

use std::collections::BTreeMap;

use uuid::Uuid;

use super::transitions;

fn awaiting(id: Uuid) -> (Uuid, (AgentState, Option<String>)) {
    (id, (AgentState::Input, Some("Grant access?".to_string())))
}

#[test]
fn a_status_the_store_already_holds_is_not_reported() {
    let id = Uuid::new_v4();
    let held = BTreeMap::from([(id, AgentState::Input)]);

    assert!(transitions(vec![awaiting(id)], |id| held.get(&id).copied()).is_empty());
}

#[test]
fn a_changed_status_is_reported_with_its_message() {
    let id = Uuid::new_v4();
    let held = BTreeMap::from([(id, AgentState::Running)]);

    assert_eq!(transitions(vec![awaiting(id)], |id| held.get(&id).copied()),
               vec![awaiting(id)]);
}

#[test]
fn one_pending_permission_across_many_ticks_reports_once() {
    let id = Uuid::new_v4();
    // The store as the tracker's sink leaves it: whatever was last reported.
    let mut held = BTreeMap::from([(id, AgentState::Running)]);
    let mut reported = 0;

    for _ in 0..30 {
        // Every tick reads the same level from the panel.
        let moved = transitions(vec![awaiting(id)], |id| held.get(&id).copied());
        reported += moved.len();
        for (id, (state, _)) in moved {
            held.insert(id, state);
        }
    }

    assert_eq!(reported, 1);
}

#[test]
fn an_agent_the_store_has_forgotten_is_dropped() {
    let held: BTreeMap<Uuid, AgentState> = BTreeMap::new();

    assert!(transitions(vec![awaiting(Uuid::new_v4())], |id| held.get(&id).copied()).is_empty());
}
