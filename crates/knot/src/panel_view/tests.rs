//! Unit tests for [`super`].

use gpui_kit::ListState;
use gpui_kit::assets::IconName;
use gpui_kit::px;
use knot_acp::PermissionRequest;

use super::style::*;
use super::summary_row::*;
use super::tool_call::*;
use super::*;
use crate::panel_state::PanelMessage;
use crate::panel_state::PanelState;

#[test]
fn every_known_kind_maps_to_a_distinct_icon_and_unknown_kinds_fall_back() {
    let known = ["read", "edit", "delete", "move", "search", "execute", "think", "fetch"];
    for kind in known {
        assert_ne!(tool_call_icon(kind),
                   IconName::Wrench,
                   "expected a specific icon for known kind {kind:?}");
    }
    assert_eq!(tool_call_icon("some-future-kind"), IconName::Wrench);
    assert_eq!(tool_call_icon(""), IconName::Wrench);
}

/// The bug this guards: keying the in-progress placeholder on "no
/// output yet" left a completed call showing "Running…" forever,
/// because a call can finish without ever reporting content.
#[test]
fn only_a_running_status_reads_as_running() {
    assert_eq!(status_label("in_progress"), "Running…");
    assert_eq!(status_label("pending"), "Pending");
    assert_eq!(status_label("completed"), "Done");
    assert_eq!(status_label("failed"), "Failed");
}

#[test]
fn an_unrecognized_status_passes_through_rather_than_vanishing() {
    assert_eq!(status_label("some_future_status"), "some_future_status");
}

#[test]
fn known_statuses_map_to_icons_and_unknown_statuses_stay_text() {
    assert_eq!(status_icon("pending"), Some(IconName::Loader));
    assert_eq!(status_icon("in_progress"), Some(IconName::Loader));
    assert_eq!(status_icon("completed"), Some(IconName::CircleCheck));
    assert_eq!(status_icon("failed"), Some(IconName::CircleX));
    assert_eq!(status_icon("some_future_status"), None);
}

#[test]
fn status_icon_color_tracks_card_outline() {
    assert_eq!(card_outline("failed"), CardOutline::Danger);
    assert_eq!(card_outline("pending"), CardOutline::Info);
    assert_eq!(card_outline("completed"), CardOutline::Neutral);
}

#[test]
fn a_failed_call_is_the_only_status_outlined_in_danger() {
    assert_eq!(card_outline("failed"), CardOutline::Danger);
    assert_eq!(card_outline("completed"), CardOutline::Neutral);
    assert_eq!(card_outline("pending"), CardOutline::Info);
    assert_eq!(card_outline("in_progress"), CardOutline::Info);
}

/// Success is the common case: outlining every finished call in a
/// colour of its own would leave the two states worth noticing - a
/// call still running and one that failed - looking like the rest.
#[test]
fn a_completed_call_recedes_to_the_neutral_border() {
    assert_eq!(card_outline("completed"), CardOutline::Neutral);
}

/// `status` is a plain wire string, so a value this build has never
/// heard of must render as an ordinary card rather than a failure -
/// or panic.
#[test]
fn an_unrecognized_status_takes_the_neutral_border() {
    assert_eq!(card_outline("some_future_status"), CardOutline::Neutral);
    assert_eq!(card_outline(""), CardOutline::Neutral);
}

#[test]
fn a_replacement_diff_reads_as_removals_then_additions() {
    let lines = diff_lines(Some("a\nb"), "a\nc");

    assert_eq!(lines,
               vec![(ERROR_COLOR, "- a".to_string()),
                    (ERROR_COLOR, "- b".to_string()),
                    (SAFE_COLOR, "+ a".to_string()),
                    (SAFE_COLOR, "+ c".to_string())]);
}

#[test]
fn a_new_file_diff_is_all_additions_with_no_prefixes_to_strip() {
    let lines = diff_lines(None, "fn main() {}");

    assert_eq!(lines, vec![(MUTED, "fn main() {}".to_string())]);
}

#[test]
fn a_unified_diff_without_old_text_colors_by_prefix() {
    let lines = diff_lines(None, "-old\n+new\n context");

    assert_eq!(lines,
               vec![(ERROR_COLOR, "- old".to_string()),
                    (SAFE_COLOR, "+ new".to_string()),
                    (MUTED, " context".to_string())]);
}

#[test]
fn the_chevron_states_which_way_the_card_goes() {
    assert_eq!(disclosure_icon(true), IconName::ChevronRight);
    assert_eq!(disclosure_icon(false), IconName::ChevronDown);
}

/// Two cards in one conversation must not share a header element id,
/// or a click on one would carry the other's state.
#[test]
fn each_tool_call_id_gets_its_own_element_id() {
    assert_eq!(element_id("tc1"), element_id("tc1"));
    assert_ne!(element_id("tc1"), element_id("tc2"));
}

#[test]
fn permission_modes_are_classified_case_insensitively() {
    assert_eq!(permission_risk_level("bypassPermissions", "Restricted"),
               RiskLevel::Danger);
    assert_eq!(permission_risk_level("unknown", "PLAN"), RiskLevel::Safe);
    assert_eq!(permission_risk_level("default", "Normal"),
               RiskLevel::Neutral);
}

fn permission_request() -> PermissionRequest {
    PermissionRequest { rpc_id:          serde_json::json!(1),
                        tool_call_id:    "tc1".to_string(),
                        tool_call_title: None,
                        options:         Vec::new(), }
}

/// The list's item count must cover the trailing permission and ended
/// cards, or tail-following would stop short of them.
#[test]
fn row_count_covers_messages_and_the_trailing_cards() {
    let mut state = PanelState::new();
    assert_eq!(row_count(&state), 0);

    state.messages.push(PanelMessage::User("hi".to_string()));
    assert_eq!(row_count(&state), 1);

    state.pending_permission = Some(permission_request());
    assert_eq!(row_count(&state), 2);

    state.ended = Some(knot_acp::SessionEndCause::ProcessExited { code: Some(1) });
    assert_eq!(row_count(&state), 3);
}

/// Indices walk the messages in order, then the permission prompt, then
/// the ended banner, and anything past the end resolves to nothing
/// rather than panicking.
#[test]
fn row_at_walks_messages_then_permission_then_ended() {
    let mut state = PanelState::new();
    state.messages.push(PanelMessage::User("a".to_string()));
    state.messages
         .push(PanelMessage::Assistant("b".to_string()));
    state.pending_permission = Some(permission_request());
    state.ended = Some(knot_acp::SessionEndCause::ProcessExited { code: None });

    assert_eq!(row_at(&state, 0), Some(PanelRow::Message(0)));
    assert_eq!(row_at(&state, 1), Some(PanelRow::Message(1)));
    assert_eq!(row_at(&state, 2), Some(PanelRow::Permission));
    assert_eq!(row_at(&state, 3), Some(PanelRow::Ended));
    assert_eq!(row_at(&state, 4), None);
}

/// The ended row is the *last* row, not tied to a fixed index: with no
/// permission pending it follows the messages directly.
#[test]
fn an_ended_row_follows_the_messages_when_no_permission_is_pending() {
    let mut state = PanelState::new();
    state.messages.push(PanelMessage::User("a".to_string()));
    state.ended = Some(knot_acp::SessionEndCause::ProcessExited { code: Some(0) });

    assert_eq!(row_count(&state), 2);
    assert_eq!(row_at(&state, 1), Some(PanelRow::Ended));
    assert_eq!(row_at(&state, 2), None);
}

/// `sync_row_count` is what keeps a live list's item count in step with
/// the state, growing and shrinking by the delta so rows already on
/// screen are left alone.
#[test]
fn sync_row_count_tracks_messages_and_trailing_rows() {
    let list = ListState::new(0, gpui_kit::ListAlignment::Top, px(LIST_OVERDRAW));
    let mut state = PanelState::new();
    let mut known = 0;

    known = sync_row_count(&list, known, &state);
    assert_eq!((list.item_count(), known), (0, 0));

    state.messages.push(PanelMessage::User("a".to_string()));
    state.messages
         .push(PanelMessage::Assistant("b".to_string()));
    known = sync_row_count(&list, known, &state);
    assert_eq!((list.item_count(), known), (2, 2));

    state.messages
         .push(PanelMessage::Assistant("c".to_string()));
    known = sync_row_count(&list, known, &state);
    assert_eq!((list.item_count(), known), (3, 3));

    state.ended = Some(knot_acp::SessionEndCause::ProcessExited { code: None });
    known = sync_row_count(&list, known, &state);
    assert_eq!((list.item_count(), known), (4, 4));

    state.messages.truncate(1);
    state.ended = None;
    known = sync_row_count(&list, known, &state);
    assert_eq!((list.item_count(), known), (1, 1));
}

/// The reconcile in `render_panel_pane` mirrors the track toggle onto
/// these two calls: `Tail` while following, `Normal` otherwise. `Normal`
/// is what keeps a toggled-off list put even at the tail, so it must not
/// report as following afterwards.
#[test]
fn follow_mode_reflects_the_toggle() {
    let list = ListState::new(0, gpui_kit::ListAlignment::Top, px(LIST_OVERDRAW));
    assert!(!list.is_following_tail());

    list.set_follow_mode(gpui_kit::FollowMode::Tail);
    assert!(list.is_following_tail());

    list.set_follow_mode(gpui_kit::FollowMode::Normal);
    assert!(!list.is_following_tail());
}

/// A conversation with a run of tool calls between two pieces of
/// assistant text, for the compact-mode row tests below.
fn conversation_with_a_run() -> PanelState {
    fn start(id: &str, kind: &str) -> knot_acp::SessionEvent {
        knot_acp::SessionEvent::Update(knot_acp::SessionUpdate::ToolCallStart {
            tool_call_id: id.to_string(),
            kind:         kind.to_string(),
            title:        String::new(),
            status:       "completed".to_string(),
            content:      Vec::new(),
            raw_input:    None,
            meta:         None,
        })
    }
    let mut state = PanelState::new();
    state.push_user_message("go".to_string());
    state.apply(start("a", "read"));
    state.apply(start("b", "execute"));
    state.apply(knot_acp::SessionEvent::Update(knot_acp::SessionUpdate::TextDelta {
        text: "done".to_string(),
    }));
    state
}

/// "Normal rendering remains available": with the preference off every
/// message draws as itself, tool calls included.
#[test]
fn compact_mode_off_draws_every_message_as_itself() {
    let state = conversation_with_a_run();
    for index in 0..state.messages.len() {
        assert_eq!(compact_row(index, &state, false),
                   CompactRow::Message,
                   "message {index} should render normally with compact mode off");
    }
}

/// "Compact rendering": with the preference on, the run's first call
/// carries the summary and the rest draw nothing. Non-tool messages are
/// untouched either way.
#[test]
fn compact_mode_on_summarizes_the_run_and_covers_the_rest() {
    let state = conversation_with_a_run();
    assert_eq!(compact_row(0, &state, true),
               CompactRow::Message,
               "the prompt");
    assert_eq!(compact_row(1, &state, true),
               CompactRow::Summary,
               "the run head");
    assert_eq!(compact_row(2, &state, true),
               CompactRow::Covered,
               "covered by it");
    assert_eq!(compact_row(3, &state, true),
               CompactRow::Message,
               "the text after");
}

/// "Disable compact mode": switching modes changes only what is drawn.
/// The same messages are still there, in the same order, with the same
/// tool-call records - nothing was consumed by either rendering.
#[test]
fn switching_modes_leaves_the_underlying_messages_untouched() {
    let state = conversation_with_a_run();
    let before = state.messages.clone();

    for index in 0..state.messages.len() {
        let _ = compact_row(index, &state, true);
        let _ = compact_row(index, &state, false);
    }

    assert_eq!(state.messages, before);
}

/// Opening a run in compact mode puts every one of its calls back to
/// individual cards - the inspection path compact mode has to keep.
#[test]
fn an_opened_run_draws_its_cards_again() {
    let mut state = conversation_with_a_run();
    state.toggle_tool_run("a".to_string());

    assert_eq!(compact_row(1, &state, true), CompactRow::Message);
    assert_eq!(compact_row(2, &state, true), CompactRow::Message);
}

/// The summary line's copy comes from the catalogue. Asserted as "the key
/// resolves", never as the English wording, so a copy edit cannot fail
/// this.
#[test]
fn every_summary_phrase_resolves_in_the_catalogue() {
    let keys = ["panel.summary.calls_one",
                "panel.summary.calls_other",
                "panel.summary.files_edited_one",
                "panel.summary.files_edited_other",
                "panel.summary.files_read_one",
                "panel.summary.files_read_other",
                "panel.summary.commands_run_one",
                "panel.summary.commands_run_other",
                "panel.summary.failures_one",
                "panel.summary.failures_other",
                "panel.summary.separator",
                "panel.summary.running_suffix",
                "settings.compact_tool_calls",
                "settings.compact_tool_calls_hint"];
    for key in keys {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalogue");
    }
}

/// The counts a run actually has appear in the line; the ones it does not
/// are omitted rather than shown as zero. Checked by substring against the
/// catalogue's own rendering, not against fixed English.
#[test]
fn the_summary_line_omits_counts_the_run_does_not_have() {
    let summary = crate::panel_state::ToolRunSummary { calls: 3,
                                                       files_read: 2,
                                                       ..Default::default() };
    let line = summary_text(summary);

    assert!(line.contains('3'), "the call count is always shown: {line}");
    assert!(line.contains('2'), "a count the run has is shown: {line}");
    assert!(!line.contains('0'),
            "a count it does not have is omitted: {line}");
}

/// A failed call keeps its own count in the line, so compact mode never
/// hides that something went wrong.
#[test]
fn a_failure_is_named_in_the_summary_line() {
    let quiet = summary_text(crate::panel_state::ToolRunSummary { calls: 2,
                                                                  ..Default::default() });
    let failed = summary_text(crate::panel_state::ToolRunSummary { calls: 2,
                                                                   failed: 1,
                                                                   ..Default::default() });

    assert_ne!(quiet, failed, "a failure has to change the line");
    assert!(failed.len() > quiet.len(),
            "it adds to it: {failed:?} vs {quiet:?}");
}

// ------------------------------------------------- the prompt's key hints

/// The `debug_bounds` key `Kbd` paints itself under, which is
/// `Keystroke::unparse` and so spells the platform modifier three ways.
/// `debug_bounds` wants a `&'static str`, so these are literals rather
/// than something built from the binding at runtime.
#[cfg(target_os = "macos")]
const ALLOW_HINT: &str = "kbd:cmd-shift-a";
#[cfg(target_os = "windows")]
const ALLOW_HINT: &str = "kbd:win-shift-a";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const ALLOW_HINT: &str = "kbd:super-shift-a";

#[cfg(target_os = "macos")]
const DENY_HINT: &str = "kbd:cmd-shift-d";
#[cfg(target_os = "windows")]
const DENY_HINT: &str = "kbd:win-shift-d";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const DENY_HINT: &str = "kbd:super-shift-d";

/// Draws the permission prompt into a real window and returns whether each
/// hint made it into the painted frame.
///
/// The prompt is rendered from inside a view rather than handed to
/// `VisualTestContext::draw`: the decision controls are interactive
/// elements, and gpui resolves those against the currently rendering view,
/// so drawing one as a bare element panics before it can paint.
///
/// `install_actions_and_keys` is what puts the bindings in the keymap the
/// prompt reads, over a temporary settings root so the persist it performs
/// cannot reach the developer's own workspaces and agents.
fn painted_hints(cx: &mut gpui_kit::TestAppContext) -> (bool, bool) {
    let dir = tempfile::tempdir().expect("failed to make a temp settings directory");
    let settings = knot_core::Settings::with_store_root(dir.path());
    std::mem::forget(dir);
    let store = std::sync::Arc::new(parking_lot::Mutex::new(knot_agents::AgentStore::new()));
    cx.update(|cx| {
          gpui_kit::init(cx);
          crate::app_bootstrap::install_actions_and_keys(&settings, store, cx);
      });

    let mut state = PanelState::new();
    state.pending_permission = Some(permission_request());
    let request = state.pending_permission.clone().expect("just set above");

    let window = cx.add_window(|_, _| PromptView { state, request });
    let mut cx = gpui_kit::VisualTestContext::from_window(window.into(), cx);
    cx.run_until_parked();

    (cx.debug_bounds(ALLOW_HINT).is_some(), cx.debug_bounds(DENY_HINT).is_some())
}

/// A view whose whole body is the permission prompt.
struct PromptView {
    state:   PanelState,
    request: PermissionRequest,
}

impl gpui_kit::Render for PromptView {
    fn render(&mut self, window: &mut gpui_kit::Window, _: &mut gpui_kit::Context<Self>)
              -> impl gpui_kit::IntoElement {
        super::render::render_permission_prompt(&self.state,
                                                &self.request,
                                                RiskLevel::Neutral,
                                                std::rc::Rc::new(|_| {}),
                                                window)
    }
}

/// The hint has to survive all the way into the painted frame, not merely
/// resolve from the keymap: `Kbd` renders nothing for an action it cannot
/// find, and `children(None)` is silently empty, so every failure between
/// the binding and the pixel looks like a button that simply has no hint.
#[gpui_kit::test]
fn the_prompt_paints_a_key_hint_on_each_decision_button(cx: &mut gpui_kit::TestAppContext) {
    let (allow, deny) = painted_hints(cx);
    assert!(allow, "the Allow button painted no {ALLOW_HINT} hint");
    assert!(deny, "the Deny button painted no {DENY_HINT} hint");
}
