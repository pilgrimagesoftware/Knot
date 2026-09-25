//! What a subagent row resolves to, before any element is built.
//!
//! Element construction needs a GPUI context; these cover the decisions that
//! do not - which is every decision the spec actually constrains.

use std::time::{Duration, Instant};

use knot_subagents::{Subagent, SubagentId, SubagentKind, SubagentState};

use super::{state_text, subagent_fields};

fn running(task: &str, kind: Option<&str>, started: Instant) -> Subagent {
    Subagent::dispatched(SubagentId::new("s1"),
                         SubagentKind::from_reported(kind),
                         task.to_owned(),
                         started)
}

#[test]
fn a_row_shows_the_kind_the_agent_named() {
    let base = Instant::now();
    let fields = subagent_fields(&running("Map the callers", Some("discovery"), base), base);

    assert_eq!(fields.kind, "discovery");
    assert_eq!(fields.task, "Map the callers");
}

/// The kind is the agent's data and is shown verbatim; the word for *having
/// no kind* is ours, so only that one is looked up.
#[test]
fn a_row_with_no_stated_kind_falls_back_to_the_catalogue() {
    let base = Instant::now();
    let fields = subagent_fields(&running("Map the callers", None, base), base);

    assert_eq!(fields.kind,
               knot_core::l10n::t("processes.subagent_kind_unstated"));
    assert!(!fields.kind.is_empty(),
            "the absent case must not render blank");
}

/// A kind that collides with a catalogue key must still render as the agent
/// wrote it - looking it up would translate the user's own configuration.
#[test]
fn a_kind_is_never_looked_up_in_the_catalogue() {
    let base = Instant::now();
    let fields = subagent_fields(&running("go", Some("processes.title"), base), base);

    assert_eq!(fields.kind, "processes.title");
}

/// The row truncates visually; the field keeps one line so the layout cannot
/// be broken by an agent that wrote a newline into its description.
#[test]
fn a_multi_line_task_is_flattened_to_one_line() {
    let base = Instant::now();
    let fields = subagent_fields(&running("first line\nsecond line", None, base), base);

    assert!(!fields.task.contains('\n'));
}

/// The same `processes.runtime_*` shapes the process rows use - a subagent's
/// elapsed time is the same kind of quantity, so it reads the same way.
#[test]
fn runtime_uses_the_sections_existing_duration_shapes() {
    let base = Instant::now();
    let subagent = running("go", None, base);

    let fields = subagent_fields(&subagent, base + Duration::from_secs(245));

    assert_eq!(fields.runtime,
               super::runtime_text(Duration::from_secs(245)));
}

#[test]
fn a_finished_subagents_runtime_stops_at_its_end() {
    let base = Instant::now();
    let mut subagent = running("go", None, base);
    subagent.complete(knot_subagents::Outcome::Succeeded,
                      None,
                      base + Duration::from_secs(30));

    let fields = subagent_fields(&subagent, base + Duration::from_secs(3600));

    assert_eq!(fields.runtime, super::runtime_text(Duration::from_secs(30)));
}

#[test]
fn every_state_resolves_to_its_own_word() {
    let running = state_text(&SubagentState::Running);
    let finished = state_text(&SubagentState::Finished);
    let failed = state_text(&SubagentState::Failed { reason: None });

    assert_eq!(running, knot_core::l10n::t("processes.subagent_running"));
    assert_eq!(finished, knot_core::l10n::t("processes.subagent_finished"));
    assert_eq!(failed, knot_core::l10n::t("processes.subagent_failed"));
    assert_ne!(finished, failed, "a failure must not read as a success");
}

/// A failure with a reason and one without are the same *state*, so the
/// label is the same; the reason rides beside the task instead.
#[test]
fn a_failures_reason_does_not_change_its_state_word() {
    let with = state_text(&SubagentState::Failed { reason: Some("no such persona".to_owned()), });
    let without = state_text(&SubagentState::Failed { reason: None });

    assert_eq!(with, without);
}
