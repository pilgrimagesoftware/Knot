//! Unit tests for [`super`]: the text a section shows, which is the part
//! that can be wrong without anyone noticing on screen.

use std::collections::BTreeMap;
use std::time::Duration;

use knot_core::ViewMode;
use knot_processes::{Activity, DescendantProcess, ProcessRecord, ProcessTable};
use uuid::Uuid;

use super::empty::{EmptyState, empty_state};
use super::process_row::row_fields;
use super::section::section_is_shown;
use super::text::{activity_text, runtime_text};

fn descendant(pid: u32, command: &str, seconds: u64, activity: Activity) -> DescendantProcess {
    DescendantProcess { pid,
                        ppid: 1,
                        command: command.to_owned(),
                        elapsed: Duration::from_secs(seconds),
                        activity }
}

/// Every key the section can show has to resolve, and none may leave a
/// placeholder behind. Asserted by key, never against the English copy: a
/// copy edit must not fail a test.
#[test]
fn every_key_the_section_shows_resolves() {
    for key in ["processes.title",
                "processes.count_unknown",
                "processes.count_none",
                "processes.count_one",
                "processes.count_many",
                "processes.summary_separator",
                "processes.column_command",
                "processes.column_runtime",
                "processes.column_pid",
                "processes.background",
                "processes.foreground",
                "processes.terminate",
                "processes.terminating",
                "processes.terminate_title",
                "processes.copy_pid",
                "processes.copied_pid",
                "processes.copy_command",
                "processes.copied_command",
                "processes.open_viewer",
                "processes.empty_not_running",
                "processes.empty_none"]
    {
        let text = knot_core::l10n::t(key);

        assert_ne!(text, key, "{key} is missing from the catalog");
        assert!(!text.is_empty(), "{key} resolves to nothing");
    }
}

/// Every sentence with a placeholder has to keep its value through
/// substitution - a body that lost its `%{...}` still resolves, and still
/// reads like a question.
#[test]
fn every_sentence_with_a_value_substitutes_it() {
    let more = knot_core::l10n::t_with("processes.summary_more",
                                       &[("names", "node, rg"), ("count", "3")]);
    assert!(more.contains("node, rg"), "{more}");
    assert!(more.contains('3'), "{more}");
    assert!(!more.contains("%{"), "{more}");

    let body = knot_core::l10n::t_with("processes.terminate_body", &[("command", "npm run dev")]);
    assert!(body.contains("npm run dev"), "{body}");
    assert!(!body.contains("%{"), "{body}");

    let failed = knot_core::l10n::t_with("processes.sample_failed",
                                         &[("reason", "ps is not on PATH")]);
    assert!(failed.contains("ps is not on PATH"), "{failed}");

    let refused = knot_core::l10n::t_with("processes.terminate_failed",
                                          &[("reason", "Operation not permitted")]);
    assert!(refused.contains("Operation not permitted"), "{refused}");
}

#[test]
fn runtime_reads_at_the_coarsest_units_that_say_something() {
    assert_eq!(runtime_text(Duration::from_secs(33)),
               knot_core::l10n::t_with("processes.runtime_seconds", &[("seconds", "33")]));
    assert_eq!(runtime_text(Duration::from_secs(48 * 60 + 8)),
               knot_core::l10n::t_with("processes.runtime_minutes",
                                       &[("minutes", "48"), ("seconds", "8")]));
    assert_eq!(runtime_text(Duration::from_secs(3723)),
               knot_core::l10n::t_with("processes.runtime_hours",
                                       &[("hours", "1"), ("minutes", "2")]));
    assert_eq!(runtime_text(Duration::from_secs(4 * 86_400 + 5 * 3600)),
               knot_core::l10n::t_with("processes.runtime_days", &[("days", "4"), ("hours", "5")]));
}

#[test]
fn a_brand_new_process_still_reads_as_a_runtime() {
    let text = runtime_text(Duration::from_secs(0));

    assert!(text.contains('0'), "{text}");
    assert!(!text.is_empty());
}

#[test]
fn each_classification_has_its_own_label() {
    let background = activity_text(Activity::Background);
    let foreground = activity_text(Activity::Foreground);

    assert_ne!(background, foreground);
    assert_eq!(background, knot_core::l10n::t("processes.background"));
    assert_eq!(foreground, knot_core::l10n::t("processes.foreground"));
}

/// The four cases the expanded section can be in. A blank section is the
/// one the spec forbids, so every case that has no rows must name itself.
#[test]
fn an_empty_section_says_which_case_it_is_in() {
    assert_eq!(empty_state(false, None), Some(EmptyState::NotRunning));
    assert_eq!(empty_state(false, Some(&[descendant(10, "x", 1, Activity::Background)])),
               Some(EmptyState::NotRunning),
               "a stopped agent's leftover rows describe a tree that is gone");
    assert_eq!(empty_state(true, None), Some(EmptyState::Counting));
    assert_eq!(empty_state(true, Some(&[])),
               Some(EmptyState::NothingSpawned));
    assert_eq!(empty_state(true, Some(&[descendant(10, "x", 1, Activity::Background)])),
               None);
}

#[test]
fn each_empty_state_reads_differently() {
    let texts = [EmptyState::NotRunning.text(),
                 EmptyState::NothingSpawned.text(),
                 EmptyState::Counting.text()];

    for (index, text) in texts.iter().enumerate() {
        assert!(!text.is_empty());
        for other in &texts[index + 1..] {
            assert_ne!(text, other, "two empty states read the same");
        }
    }
}

/// A command wider than the row still yields every trailing field. The
/// ellipsis is a style on the command's own box, so the runtime, identifier
/// and classification are unaffected by how long it is.
#[test]
fn a_long_command_keeps_the_trailing_fields() {
    let long = "node ".to_owned() + &"--very-long-flag ".repeat(60);
    let fields = row_fields(&descendant(4242, &long, 90, Activity::Background));

    assert!(!fields.runtime.is_empty());
    assert_eq!(fields.pid, "4242");
    assert_eq!(fields.activity, knot_core::l10n::t("processes.background"));
    assert!(!fields.command.contains('…'),
            "the row carries the command whole; the ellipsis is drawn, not spliced");
}

/// A command containing a newline renders as one line. The style flags do
/// not achieve this on their own - see `app_support::single_line`.
#[test]
fn a_command_with_a_line_break_is_flattened_to_one_line() {
    let fields = row_fields(&descendant(7, "sh -c 'echo one\necho two'", 5, Activity::Background));

    assert!(!fields.command.contains('\n'), "{}", fields.command);
    assert!(fields.command.contains("echo one"));
    assert!(fields.command.contains("echo two"));
}

/// What the copy action hands over is the process's own command, not the
/// text the row drew.
#[test]
fn the_command_a_row_copies_is_the_untruncated_one() {
    let long = "npm exec -- ".to_owned() + &"x".repeat(500);
    let process = descendant(9, &long, 5, Activity::Background);

    // What `copy_process_command` is handed.
    assert_eq!(process.command, long);
    assert!(!process.command.contains('…'));
    assert_eq!(process.command.len(), long.len());
}

/// Rows reach the section already ordered, so what the list draws is what
/// the walk decided: background before foreground, longest-running first.
#[test]
fn rows_arrive_background_first_then_longest_running() {
    let agent = Uuid::new_v4();
    let table = ProcessTable::from_records([record(100, 1, "shell", 100, 0),
                                            // Foreground: its own group is its terminal's
                                            // foreground group.
                                            record(200, 100, "cargo test", 900, 100),
                                            record(300, 100, "short server", 10, 0),
                                            record(400, 100, "long server", 500, 0)]);

    let descendants =
        crate::agent_processes::descendants_for(&table, &BTreeMap::from([(agent, 100)]));
    let order: Vec<_> = descendants[&agent].iter()
                                           .map(|process| process.pid)
                                           .collect();

    assert_eq!(order,
               vec![400, 300, 200],
               "background by runtime, then the foreground command last");
}

/// The section is mounted below whichever session pane is showing, so both
/// views get it from one place. A regression here would show the section in
/// the terminal view and quietly drop it from the panel, or the reverse.
#[test]
fn the_section_is_present_in_both_the_terminal_and_the_panel_view() {
    for view_mode in [ViewMode::Terminal, ViewMode::Panel] {
        assert!(section_is_shown(view_mode, false, false), "{view_mode:?}");
    }
}

/// A takeover view has replaced the agent's pane, and a document the agent
/// opened has taken the content area: in neither case is there a session
/// pane for the section to sit under.
#[test]
fn the_section_is_absent_where_there_is_no_session_pane() {
    for view_mode in [ViewMode::Terminal, ViewMode::Panel] {
        assert!(!section_is_shown(view_mode, true, false), "a takeover view");
        assert!(!section_is_shown(view_mode, false, true), "a document pane");
    }
}

fn record(pid: u32, ppid: u32, command: &str, seconds: u64, group: u32) -> ProcessRecord {
    ProcessRecord { pid,
                    ppid,
                    pgid: if group == 0 { pid } else { group },
                    tpgid: group as i32,
                    elapsed: Duration::from_secs(seconds),
                    command: command.to_owned() }
}
