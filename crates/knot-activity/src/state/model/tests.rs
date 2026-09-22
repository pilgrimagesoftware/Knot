//! Unit tests for [`super`].

use std::time::Instant as StdInstant;

use super::*;
use crate::tracking::tracking_for;

fn at(offset: Duration) -> Instant {
    Instant::from_std(StdInstant::now()) + offset
}

fn config(agent_type: &str) -> TrackerConfig {
    TrackerConfig::for_agent_type(agent_type)
}

fn status_effects(effects: &[Effect]) -> Vec<AgentState> {
    effects.iter()
           .filter_map(|e| match e {
               Effect::Status(event) => Some(event.status),
               _ => None,
           })
           .collect()
}

fn has_effect(effects: &[Effect], kind: EffectKind) -> bool {
    effects.iter().any(|e| e.kind() == kind)
}

// Local mirror of Effect discriminants for assertions.
#[derive(PartialEq)]
enum EffectKind {
    Status,
    AwaitingInput,
    CheckMessages,
    InjectRegistration,
}

trait EffectTag {
    fn kind(&self) -> EffectKind;
}
impl EffectTag for Effect {
    fn kind(&self) -> EffectKind {
        match self {
            Effect::Status(_) => EffectKind::Status,
            Effect::AwaitingInput(_) => EffectKind::AwaitingInput,
            Effect::CheckMessages => EffectKind::CheckMessages,
            Effect::InjectRegistration(_) => EffectKind::InjectRegistration,
        }
    }
}

#[test]
fn shell_agent_never_leaves_idle() {
    let mut state = ActivityState::new(config("shell"),
                                       tracking_for("shell", knot_core::ViewMode::Terminal));
    let effects = state.on_terminal_activity(at(Duration::ZERO));
    assert!(status_effects(&effects).is_empty());
    assert_eq!(state.status(), AgentState::Idle);
    assert!(state.idle_due.is_none());
}

#[test]
fn panel_mode_agent_ignores_terminal_output_and_keystrokes() {
    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Panel));
    let t0 = at(Duration::ZERO);

    let output_effects = state.on_terminal_activity(t0);
    let input_effects = state.on_user_input(t0, KeyEvent::Other);

    assert!(status_effects(&output_effects).is_empty());
    assert!(status_effects(&input_effects).is_empty());
    assert_eq!(state.status(), AgentState::Idle);
    assert!(state.idle_due.is_none());
    assert!(state.guard_deadline().is_none());
}

#[test]
fn acp_turn_start_moves_to_working() {
    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Panel));
    let t0 = at(Duration::ZERO);

    let effects = state.apply_acp_status(t0, AgentState::Running, None);

    assert_eq!(state.status(), AgentState::Running);
    assert_eq!(status_effects(&effects), vec![AgentState::Running]);
    assert!(state.idle_deadline().is_none(),
            "ACP transitions never arm the idle timer");
}

#[test]
fn acp_turn_end_with_no_pending_permission_moves_to_idle() {
    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Panel));
    let t0 = at(Duration::ZERO);
    state.apply_acp_status(t0, AgentState::Running, None);

    let effects = state.apply_acp_status(t0, AgentState::Idle, None);

    assert_eq!(state.status(), AgentState::Idle);
    assert_eq!(status_effects(&effects), vec![AgentState::Idle]);
}

#[test]
fn acp_permission_request_moves_to_awaiting_input_immediately() {
    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Panel));
    let t0 = at(Duration::ZERO);
    state.apply_acp_status(t0, AgentState::Running, None);

    let effects = state.apply_acp_status(t0, AgentState::Input, Some("allow tool X?".to_string()));

    assert_eq!(state.status(), AgentState::Input);
    assert_eq!(status_effects(&effects), vec![AgentState::Input]);
    assert!(effects.iter().any(|e| matches!(
                              e,
                              Effect::AwaitingInput(Some(message)) if message == "allow tool X?"
                          )));
}

#[test]
fn acp_session_error_moves_to_error() {
    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Panel));
    let t0 = at(Duration::ZERO);
    state.apply_acp_status(t0, AgentState::Running, None);

    let effects = state.apply_acp_status(t0, AgentState::Error, None);

    assert_eq!(state.status(), AgentState::Error);
    assert_eq!(status_effects(&effects), vec![AgentState::Error]);
}

#[test]
fn terminal_output_sets_working_and_arms_idle_timer() {
    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Terminal));
    let t0 = at(Duration::ZERO);
    let effects = state.on_terminal_activity(t0);
    assert_eq!(state.status(), AgentState::Running);
    assert_eq!(status_effects(&effects), vec![AgentState::Running]);
    assert_eq!(state.idle_deadline(), Some(t0 + Duration::from_secs(3)));
}

#[test]
fn idle_fires_after_quiet_period() {
    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Terminal));
    let t0 = at(Duration::ZERO);
    state.on_terminal_activity(t0);
    let effects = state.idle_timer_fired(t0 + Duration::from_secs(3));
    assert_eq!(state.status(), AgentState::Idle);
    assert!(has_effect(&effects, EffectKind::CheckMessages));
}

#[test]
fn late_activity_defers_idle_for_remaining_interval() {
    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Terminal));
    let t0 = at(Duration::ZERO);
    state.on_terminal_activity(t0);
    state.on_terminal_activity(t0 + Duration::from_secs(2));
    let effects = state.idle_timer_fired(t0 + Duration::from_secs(3));
    assert!(status_effects(&effects).is_empty());
    assert_eq!(state.status(), AgentState::Running);
    assert_eq!(state.idle_deadline(), Some(t0 + Duration::from_secs(5)));
}

#[test]
fn runtime_downgrade_stops_terminal_output_transitions() {
    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Terminal));
    let t0 = at(Duration::ZERO);
    state.on_terminal_activity(t0);
    assert_eq!(state.status(), AgentState::Running);
    state.set_tracking(ActivityTracking::USER_INPUT);
    let effects = state.on_terminal_activity(t0 + Duration::from_secs(1));
    assert!(status_effects(&effects).is_empty());
    assert_eq!(state.status(), AgentState::Running);
    assert!(state.idle_due.is_none(),
            "output with tracking removed cancels idle timer");
}

#[test]
fn user_input_in_plain_agent_shows_working_with_ten_second_idle() {
    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Terminal));
    let t0 = at(Duration::ZERO);
    state.on_user_input(t0, KeyEvent::Other);
    assert_eq!(state.status(), AgentState::Running);
    assert_eq!(state.idle_deadline(), Some(t0 + Duration::from_secs(10)));
    assert_eq!(state.guard_deadline(), Some(t0 + Duration::from_secs(10)));
}

#[test]
fn user_input_in_hook_agent_does_not_flip_status() {
    let mut cfg = config("claude");
    cfg.is_hook_based = true;
    let mut state = ActivityState::new(cfg, tracking_for("claude", knot_core::ViewMode::Terminal));
    let t0 = at(Duration::ZERO);
    state.on_user_input(t0, KeyEvent::Other);
    assert_eq!(state.status(), AgentState::Idle);
    assert_eq!(state.guard_deadline(), Some(t0 + Duration::from_secs(10)));
}

#[test]
fn guard_expiry_triggers_message_check() {
    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Terminal));
    let t0 = at(Duration::ZERO);
    state.on_user_input(t0, KeyEvent::Other);
    let effects = state.guard_expired(t0 + Duration::from_secs(10));
    assert_eq!(state.guard_deadline(), None);
    assert!(has_effect(&effects, EffectKind::CheckMessages));
}

#[test]
fn hook_awaiting_input_raises_notification() {
    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Terminal));
    let t0 = at(Duration::ZERO);
    let effects = state.apply_hook_status(t0, AgentState::Input, Some("grant access?".into()));
    assert_eq!(state.status(), AgentState::Input);
    assert_eq!(effects,
               vec![Effect::Status(StatusEvent { status:     AgentState::Input,
                                                 source:     ActivitySource::Hook,
                                                 changed_at: t0, }),
                    Effect::AwaitingInput(Some("grant access?".into())),]);
}

#[test]
fn return_answers_prompt_and_escape_dismisses() {
    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Terminal));
    let t0 = at(Duration::ZERO);
    state.apply_hook_status(t0, AgentState::Input, None);
    state.on_user_input(t0 + Duration::from_secs(1), KeyEvent::Return);
    assert_eq!(state.status(), AgentState::Running);

    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Terminal));
    let t0 = at(Duration::ZERO);
    state.apply_hook_status(t0, AgentState::Input, None);
    state.on_user_input(t0 + Duration::from_secs(1), KeyEvent::Escape);
    assert_eq!(state.status(), AgentState::Idle);
}

#[test]
fn idle_timers_do_not_move_awaiting_input_agent() {
    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Terminal));
    let t0 = at(Duration::ZERO);
    state.apply_hook_status(t0, AgentState::Input, None);
    let effects = state.idle_timer_fired(t0 + Duration::from_secs(60));
    assert!(effects.is_empty());
    assert_eq!(state.status(), AgentState::Input);
}

#[test]
fn hook_idle_overrides_local_working_and_cancels_guard() {
    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Terminal));
    let t0 = at(Duration::ZERO);
    state.on_user_input(t0, KeyEvent::Other);
    assert_eq!(state.status(), AgentState::Running);
    let effects = state.apply_hook_status(t0 + Duration::from_secs(1), AgentState::Idle, None);
    assert_eq!(status_effects(&effects), vec![AgentState::Idle]);
    assert_eq!(state.guard_deadline(), None);
}

#[test]
fn process_exit_sets_error_or_idle() {
    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Terminal));
    let t0 = at(Duration::ZERO);
    state.on_process_exit(t0, Some(1));
    assert_eq!(state.status(), AgentState::Error);
    assert!(state.idle_due.is_none());

    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Terminal));
    let t0 = at(Duration::ZERO);
    state.on_process_exit(t0, Some(0));
    assert_eq!(state.status(), AgentState::Idle);

    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Terminal));
    let t0 = at(Duration::ZERO);
    state.on_process_exit(t0, None);
    assert_eq!(state.status(), AgentState::Idle);
}

#[test]
fn registration_waits_for_first_idle() {
    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Terminal));
    let t0 = at(Duration::ZERO);
    state.start(t0);
    state.set_registration_prompt("Register with the knot".into());
    let effects = state.registration_ready_fired(t0 + Duration::from_secs(2));
    assert!(effects.is_empty(), "cannot inject before first idle");
    assert_eq!(state.status(), AgentState::Idle);
}

#[test]
fn registration_injected_once_after_idle() {
    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Terminal));
    let t0 = at(Duration::ZERO);
    state.start(t0);
    state.set_registration_prompt("Register with the knot".into());
    state.on_terminal_activity(t0 + Duration::from_millis(100));
    let t_idle = t0 + Duration::from_secs(4);
    state.idle_timer_fired(t_idle);
    let effects = state.registration_ready_fired(t_idle + Duration::from_secs(2));
    assert_eq!(effects,
               vec![Effect::InjectRegistration("Register with the knot".into())]);
    let effects = state.registration_ready_fired(t_idle + Duration::from_secs(3));
    assert!(effects.is_empty(), "injected only once");
}

#[test]
fn registration_was_not_injected_when_guard_blocked() {
    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Terminal));
    let t0 = at(Duration::ZERO);
    state.start(t0);
    state.set_registration_prompt("Register with the knot".into());
    // Reach Idle so the ready gate opens, then type to arm the guard.
    state.on_terminal_activity(t0 + Duration::from_millis(100));
    state.idle_timer_fired(t0 + Duration::from_secs(4));
    state.on_user_input(t0 + Duration::from_secs(5), KeyEvent::Other);
    let effects = state.registration_ready_fired(t0 + Duration::from_secs(6));
    assert!(effects.is_empty(), "guard blocks injection");
    // Guard expiry re-evaluates and injects.
    let effects = state.guard_expired(t0 + Duration::from_secs(15));
    assert!(has_effect(&effects, EffectKind::InjectRegistration));
}

#[test]
fn hook_fallback_timeout_used_for_hook_agents() {
    let mut cfg = config("claude");
    cfg.is_hook_based = true;
    let mut state = ActivityState::new(cfg, tracking_for("claude", knot_core::ViewMode::Terminal));
    let t0 = at(Duration::ZERO);
    state.on_terminal_activity(t0);
    assert_eq!(state.idle_deadline(), Some(t0 + Duration::from_secs(5)));
}

#[test]
fn idle_transition_records_change_time() {
    let mut state = ActivityState::new(config("claude"),
                                       tracking_for("claude", knot_core::ViewMode::Terminal));
    let t0 = at(Duration::ZERO);
    state.on_terminal_activity(t0);
    let changed = state.changed_at();
    assert_eq!(changed, t0);
    state.mark_idle_for_test(t0 + Duration::from_secs(3));
    assert_eq!(state.changed_at(), t0 + Duration::from_secs(3));
}

impl ActivityState {
    fn mark_idle_for_test(&mut self, now: Instant) {
        let mut effects = Vec::new();
        self.mark_idle(now, ActivitySource::Terminal, &mut effects);
    }
}

#[test]
fn shell_config_disables_registration() {
    // Shell registers inline (like the launch crate's capability table),
    // so no deferred registration is scheduled.
    let mut cfg = config("shell");
    cfg.inline_registration = true;
    let mut state = ActivityState::new(cfg, tracking_for("shell", knot_core::ViewMode::Terminal));
    let t0 = at(Duration::ZERO);
    state.start(t0);
    assert!(state.registration_deadline().is_none());
}
