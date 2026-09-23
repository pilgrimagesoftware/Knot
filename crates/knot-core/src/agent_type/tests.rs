//! Unit tests for [`super`].

use super::*;

/// An id typed twice is two types that behave as one, which is exactly the
/// confusion the roster exists to prevent.
#[test]
fn every_id_appears_once() {
    let mut ids = ALL.iter()
                     .map(|agent_type| agent_type.id)
                     .collect::<Vec<_>>();
    ids.sort_unstable();
    let unique = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), unique);
}

#[test]
fn the_default_and_shell_ids_are_in_the_roster() {
    assert!(info(DEFAULT).is_some());
    assert!(is_shell(SHELL));
}

/// Exactly one type is a shell: the view mode, the launch path and the
/// tracking preset all branch on it, so a second one would have to be
/// taught to each of them separately.
#[test]
fn only_one_type_is_a_shell() {
    assert_eq!(ALL.iter().filter(|agent_type| agent_type.is_shell).count(),
               1);
}

/// A custom type is a user's own command, so nothing may assume vendor
/// behaviour of it - no inline registration, no hook-driven activity.
#[test]
fn a_custom_type_claims_no_vendor_behaviour() {
    for agent_type in ALL.iter().filter(|agent_type| agent_type.is_custom) {
        assert!(!agent_type.inline_registration,
                "{} claims inline registration",
                agent_type.id);
        assert!(!agent_type.hook_activity,
                "{} claims hook activity",
                agent_type.id);
    }
}

/// An unrecognized type is a working configuration - it launches through
/// the terminal path - so every lookup has to answer for one rather than
/// panicking or pretending it is the default.
#[test]
fn an_unknown_type_answers_as_itself() {
    assert_eq!(info("nothing-by-that-name"), None);
    assert_eq!(label("nothing-by-that-name"), "nothing-by-that-name");
    assert!(!is_shell("nothing-by-that-name"));
    assert!(!supports_inline_registration("nothing-by-that-name"));
    assert!(!has_hook_activity("nothing-by-that-name"));
    assert_eq!(subagent_reporting("nothing-by-that-name"),
               SubagentReporting::None);
}

/// The column that decides whether the processes section shows a subagents
/// group at all. A row left off the roster would read as "dispatched none",
/// which is the one answer this capability must never give by accident.
#[test]
fn every_type_states_how_it_reports_subagents() {
    for agent_type in ALL {
        // Reading the field is the assertion: the struct has no default, so a
        // row that omitted it would not compile. This test exists so that
        // removing the column from a populated row fails here too, rather
        // than only wherever the column happens to be read.
        let _ = agent_type.subagents;
    }

    assert_eq!(subagent_reporting("claude"), SubagentReporting::ToolCalls);
}

/// A shell agent has no AI and so nothing to delegate. It must resolve to
/// "cannot tell" structurally, not by anybody remembering to special-case it.
#[test]
fn a_shell_type_reports_no_subagents_in_either_view_mode() {
    assert_eq!(subagent_reporting(SHELL), SubagentReporting::None);
    assert!(!reports_subagents(SHELL, ViewMode::Terminal));
    assert!(!reports_subagents(SHELL, ViewMode::Panel));
}

/// The sequencing decision from design.md. Claude ships as `ToolCalls`, not
/// `Either`, because the hook emitter is a plugin outside this repo - so a
/// Terminal-mode Claude agent must read as *unavailable* rather than
/// confidently claiming it dispatched nothing.
#[test]
fn claude_reports_subagents_in_panel_mode_only_until_the_hook_plugin_ships() {
    assert!(reports_subagents("claude", ViewMode::Panel));
    assert!(!reports_subagents("claude", ViewMode::Terminal));
}

#[test]
fn each_reporting_path_answers_for_its_own_view_mode() {
    assert!(SubagentReporting::ToolCalls.can_report(ViewMode::Panel));
    assert!(!SubagentReporting::ToolCalls.can_report(ViewMode::Terminal));

    assert!(SubagentReporting::Hooks.can_report(ViewMode::Terminal));
    assert!(!SubagentReporting::Hooks.can_report(ViewMode::Panel));

    assert!(SubagentReporting::Either.can_report(ViewMode::Panel));
    assert!(SubagentReporting::Either.can_report(ViewMode::Terminal));

    assert!(!SubagentReporting::None.can_report(ViewMode::Panel));
    assert!(!SubagentReporting::None.can_report(ViewMode::Terminal));
}

#[test]
fn a_known_type_answers_from_its_row() {
    assert_eq!(label("opencode"), "OpenCode");
    assert!(supports_inline_registration("gemini"));
    assert!(has_hook_activity("claude"));
    assert!(!has_hook_activity("gemini"));
}
