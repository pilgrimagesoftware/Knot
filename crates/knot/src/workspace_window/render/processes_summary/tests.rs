//! Unit tests for [`super`]. Pure over a built process list - no window, no
//! `ps`, which is the same separation the section's own sampler keeps.

use std::time::Duration;

use knot_processes::{Activity, DescendantProcess};

use super::{MAX_NAMES, process_name, summary_text};

fn process(command: &str) -> DescendantProcess {
    DescendantProcess { pid:      4242,
                        ppid:     1,
                        command:  command.to_string(),
                        elapsed:  Duration::from_secs(30),
                        activity: Activity::Background, }
}

fn processes(commands: &[&str]) -> Vec<DescendantProcess> {
    commands.iter().copied().map(process).collect()
}

fn none() -> String {
    knot_core::l10n::t("processes.count_none")
}

fn counting() -> String {
    knot_core::l10n::t("processes.count_unknown")
}

// ------------------------------------------------------------ process names

#[test]
fn a_name_is_the_leading_word_without_its_directories() {
    assert_eq!(process_name("/opt/homebrew/bin/node --inspect server.js"),
               "node");
    assert_eq!(process_name("node"), "node");
    assert_eq!(process_name("/usr/bin/python3"), "python3");
}

#[test]
fn a_command_with_no_arguments_still_names() {
    assert_eq!(process_name("rg"), "rg");
}

#[test]
fn an_empty_command_names_nothing_rather_than_panicking() {
    assert_eq!(process_name(""), "");
    assert_eq!(process_name("   "), "");
}

/// A trailing slash would otherwise leave an empty name.
#[test]
fn a_trailing_slash_falls_back_to_the_whole_word() {
    assert_eq!(process_name("/usr/bin/"), "/usr/bin/");
}

// ---------------------------------------------------------------- collapsed

/// The bug: this header read "Counting…" whatever was running.
#[test]
fn collapsed_names_what_is_running() {
    let list = processes(&["/usr/bin/node server.js", "/usr/bin/rg pattern"]);
    let summary = summary_text(true, false, Some(&list));

    assert!(summary.contains("node"), "{summary}");
    assert!(summary.contains("rg"), "{summary}");
    assert_ne!(summary, counting());
}

#[test]
fn collapsed_folds_repeats_of_one_name_together() {
    let list = processes(&["node a.js", "node b.js", "node c.js"]);
    let summary = summary_text(true, false, Some(&list));

    assert_eq!(summary.matches("node").count(), 1, "{summary}");
}

/// The remainder counts processes, not the names it stood in for.
#[test]
fn collapsed_summarizes_beyond_the_shown_names() {
    let list = processes(&["node a", "rg b", "python c", "ruby d", "perl e"]);
    let summary = summary_text(true, false, Some(&list));

    assert!(summary.contains('2'),
            "expected a remainder of 2 beyond {MAX_NAMES} names: {summary}");
}

#[test]
fn collapsed_with_exactly_the_shown_names_has_no_remainder() {
    let list = processes(&["node a", "rg b", "python c"]);
    let summary = summary_text(true, false, Some(&list));
    let more = knot_core::l10n::t_with("processes.summary_more", &[("names", ""), ("count", "0")]);

    assert!(!summary.contains(more.trim()),
            "unexpected remainder: {summary}");
}

// ----------------------------------------------------------------- expanded

#[test]
fn expanded_counts_instead_of_naming() {
    let list = processes(&["node a", "rg b", "python c"]);
    let summary = summary_text(true, true, Some(&list));

    assert!(summary.contains('3'), "{summary}");
    assert!(!summary.contains("node"),
            "expanded should not name: {summary}");
}

#[test]
fn expanded_counts_one_with_the_singular_noun() {
    let list = processes(&["node a"]);

    assert_eq!(summary_text(true, true, Some(&list)),
               knot_core::l10n::pluralize(1, "processes.count_one", "processes.count_many"));
}

// -------------------------------------------------------------------- empty

#[test]
fn nothing_running_reads_as_none_in_both_states() {
    assert_eq!(summary_text(true, false, Some(&[])), none());
    assert_eq!(summary_text(true, true, Some(&[])), none());
}

/// A stopped agent has nothing running whatever its last sample said, so the
/// header must not go on naming processes the body calls gone.
#[test]
fn a_stopped_agent_reads_as_none_even_holding_a_stale_sample() {
    let list = processes(&["node a", "rg b"]);

    assert_eq!(summary_text(false, false, Some(&list)), none());
    assert_eq!(summary_text(false, true, Some(&list)), none());
}

/// Still "Counting…", but only while it is true: the sampler now runs for a
/// shown section, so this is the gap before the first sample, not forever.
#[test]
fn a_running_agent_with_no_sample_yet_is_counting() {
    assert_eq!(summary_text(true, false, None), counting());
    assert_eq!(summary_text(true, true, None), counting());
}

#[test]
fn counting_and_none_are_different_things_to_say() {
    assert_ne!(counting(), none());
}

// ----------------------------------------------------------------- catalog

/// Every branch resolves its key and leaves no placeholder behind. Asserts
/// the keys resolve, never the English copy.
#[test]
fn every_summary_resolves_from_the_catalog() {
    let list = processes(&["node a", "rg b", "python c", "ruby d", "perl e"]);
    let summaries = [summary_text(true, false, None),
                     summary_text(true, false, Some(&[])),
                     summary_text(false, false, Some(&list)),
                     summary_text(true, false, Some(&list)),
                     summary_text(true, true, Some(&list))];

    for summary in summaries {
        assert!(!summary.contains("processes."),
                "key did not resolve: {summary}");
        assert!(!summary.contains("%{"),
                "placeholder left behind: {summary}");
        assert!(!summary.is_empty(), "a summary must say something");
    }
}
