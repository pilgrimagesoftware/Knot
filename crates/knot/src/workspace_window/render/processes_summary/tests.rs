//! Unit tests for [`super`]. Pure over a built process list - no window, no
//! `ps`, which is the same separation the section's own sampler keeps.

use std::time::Duration;
use std::time::Instant;

use knot_processes::{Activity, DescendantProcess};
use knot_subagents::{Subagent, SubagentId, SubagentKind};

use super::{MAX_NAMES, Summary, process_name, summary_text};

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

/// A summary for an agent whose type cannot report subagents - which is what
/// every test written before the section had two groups was describing.
fn only_processes(is_running: bool, expanded: bool, processes: Option<&[DescendantProcess]>)
                  -> String {
    summary_text(&Summary { is_running,
                            expanded,
                            processes,
                            subagents: None })
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
    let summary = only_processes(true, false, Some(&list));

    assert!(summary.contains("node"), "{summary}");
    assert!(summary.contains("rg"), "{summary}");
    assert_ne!(summary, counting());
}

#[test]
fn collapsed_folds_repeats_of_one_name_together() {
    let list = processes(&["node a.js", "node b.js", "node c.js"]);
    let summary = only_processes(true, false, Some(&list));

    assert_eq!(summary.matches("node").count(), 1, "{summary}");
}

/// The remainder counts processes, not the names it stood in for.
#[test]
fn collapsed_summarizes_beyond_the_shown_names() {
    let list = processes(&["node a", "rg b", "python c", "ruby d", "perl e"]);
    let summary = only_processes(true, false, Some(&list));

    assert!(summary.contains('2'),
            "expected a remainder of 2 beyond {MAX_NAMES} names: {summary}");
}

#[test]
fn collapsed_with_exactly_the_shown_names_has_no_remainder() {
    let list = processes(&["node a", "rg b", "python c"]);
    let summary = only_processes(true, false, Some(&list));
    let more = knot_core::l10n::t_with("processes.summary_more", &[("names", ""), ("count", "0")]);

    assert!(!summary.contains(more.trim()),
            "unexpected remainder: {summary}");
}

// ----------------------------------------------------------------- expanded

#[test]
fn expanded_counts_instead_of_naming() {
    let list = processes(&["node a", "rg b", "python c"]);
    let summary = only_processes(true, true, Some(&list));

    assert!(summary.contains('3'), "{summary}");
    assert!(!summary.contains("node"),
            "expanded should not name: {summary}");
}

#[test]
fn expanded_counts_one_with_the_singular_noun() {
    let list = processes(&["node a"]);

    assert_eq!(only_processes(true, true, Some(&list)),
               knot_core::l10n::pluralize(1, "processes.count_one", "processes.count_many"));
}

// -------------------------------------------------------------------- empty

#[test]
fn nothing_running_reads_as_none_in_both_states() {
    assert_eq!(only_processes(true, false, Some(&[])), none());
    assert_eq!(only_processes(true, true, Some(&[])), none());
}

/// A stopped agent has nothing running whatever its last sample said, so the
/// header must not go on naming processes the body calls gone.
#[test]
fn a_stopped_agent_reads_as_none_even_holding_a_stale_sample() {
    let list = processes(&["node a", "rg b"]);

    assert_eq!(only_processes(false, false, Some(&list)), none());
    assert_eq!(only_processes(false, true, Some(&list)), none());
}

/// Still "Counting…", but only while it is true: the sampler now runs for a
/// shown section, so this is the gap before the first sample, not forever.
#[test]
fn a_running_agent_with_no_sample_yet_is_counting() {
    assert_eq!(only_processes(true, false, None), counting());
    assert_eq!(only_processes(true, true, None), counting());
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
    let summaries = [only_processes(true, false, None),
                     only_processes(true, false, Some(&[])),
                     only_processes(false, false, Some(&list)),
                     only_processes(true, false, Some(&list)),
                     only_processes(true, true, Some(&list))];

    for summary in summaries {
        assert!(!summary.contains("processes."),
                "key did not resolve: {summary}");
        assert!(!summary.contains("%{"),
                "placeholder left behind: {summary}");
        assert!(!summary.is_empty(), "a summary must say something");
    }
}

// ------------------------------------------------------------ two groups

fn subagent(kind: &str) -> Subagent {
    Subagent::dispatched(SubagentId::new(kind),
                         SubagentKind::from_reported(Some(kind)),
                         format!("task for {kind}"),
                         Instant::now())
}

fn subagents(kinds: &[&str]) -> Vec<Subagent> {
    kinds.iter().copied().map(subagent).collect()
}

fn both(expanded: bool, subagents: &[Subagent], processes: &[DescendantProcess]) -> String {
    summary_text(&Summary { is_running: true,
                            expanded,
                            processes: Some(processes),
                            subagents: Some(subagents) })
}

/// Delegation is the fact a user cannot get anywhere else in Knot - a `node`
/// under an agent is visible from Activity Monitor, and "waiting on three
/// subagents" is not. So it comes first.
#[test]
fn a_collapsed_header_names_subagent_kinds_before_process_names() {
    let text = both(false, &subagents(&["discovery"]), &processes(&["node"]));

    let discovery = text.find("discovery").expect("names the subagent kind");
    let node = text.find("node").expect("names the process");

    assert!(discovery < node,
            "expected the subagent kind first, got {text:?}");
}

/// The remainder counts what the listed names do *not* cover, so three
/// subagents of one kind under one listed name leave nothing over. Same rule
/// the eight-`node` case follows.
#[test]
fn repeats_of_one_subagent_kind_are_folded_together() {
    let list = subagents(&["code-review", "code-review", "code-review"]);

    let text = both(false, &list, &[]);

    assert_eq!(text, "code-review");
}

#[test]
fn the_remainder_counts_both_kinds_the_names_do_not_cover() {
    let list = subagents(&["a", "b", "c", "d"]);
    let running = processes(&["node", "rg"]);

    let text = both(false, &list, &running);

    // Three names shown, so one subagent and both processes are uncovered.
    assert!(text.contains('3'),
            "expected a remainder of three, got {text:?}");
}

#[test]
fn an_expanded_header_counts_each_group_separately() {
    let text = both(true,
                    &subagents(&["a", "b"]),
                    &processes(&["node", "rg", "cargo"]));

    assert!(text.contains('2'),
            "expected the subagent count, got {text:?}");
    assert!(text.contains('3'),
            "expected the process count, got {text:?}");
    assert!(!text.contains('5'),
            "the two counts must not be added together: {text:?}");
}

/// A group with nothing in it is left out rather than counted as zero: the
/// group below already says so in words, and "0 subagents, 3 processes"
/// spends the header's width on the half with nothing to report.
#[test]
fn an_expanded_header_omits_a_group_that_is_empty() {
    let text = both(true, &[], &processes(&["node"]));

    assert!(!text.contains('0'), "expected no zero count, got {text:?}");
    assert_eq!(text,
               knot_core::l10n::pluralize(1, "processes.count_one", "processes.count_many"));
}

/// `None` and `Some(&[])` are different answers. A type that cannot report
/// draws no subagents group, so the header must not claim a count for it -
/// and must read exactly as it did before the section had two groups.
#[test]
fn an_unreportable_type_reads_as_it_did_before_the_second_group() {
    let running = processes(&["node"]);

    let unreportable = summary_text(&Summary { is_running: true,
                                               expanded:   true,
                                               processes:  Some(&running),
                                               subagents:  None, });

    assert_eq!(unreportable, only_processes(true, true, Some(&running)));
}

/// An agent that has stopped has nothing running whatever its last records
/// held - the header must not contradict the body's "not running".
#[test]
fn a_stopped_agent_does_not_keep_naming_its_subagents() {
    let list = subagents(&["discovery"]);

    let text = summary_text(&Summary { is_running: false,
                                       expanded:   false,
                                       processes:  Some(&processes(&["node"])),
                                       subagents:  Some(&list), });

    assert_eq!(text, none());
}

/// Subagents are not sampled, so they are never the thing being waited on. A
/// held subagent must not rescue the header from the unknown state while the
/// first process sample is still outstanding.
#[test]
fn a_subagent_does_not_hide_a_missing_first_sample() {
    let list = subagents(&["discovery"]);

    let text = summary_text(&Summary { is_running: true,
                                       expanded:   false,
                                       processes:  None,
                                       subagents:  Some(&list), });

    assert_eq!(text, counting());
}

#[test]
fn nothing_running_of_either_kind_reads_as_none() {
    assert_eq!(both(false, &[], &[]), none());
    assert_eq!(both(true, &[], &[]), none());
}

/// A subagent that named no kind still needs a word in the header, and it is
/// the catalogue's rather than blank.
#[test]
fn a_subagent_with_no_kind_is_named_from_the_catalogue() {
    let unstated = Subagent::dispatched(SubagentId::new("s1"),
                                        SubagentKind::Unstated,
                                        "go".to_owned(),
                                        Instant::now());

    let text = both(false, &[unstated], &[]);

    assert_eq!(text, knot_core::l10n::t("processes.subagent_kind_unstated"));
}
