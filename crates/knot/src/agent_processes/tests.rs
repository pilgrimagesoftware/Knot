//! Unit tests for [`super`].

use std::cell::Cell;
use std::collections::BTreeMap;
use std::time::Duration;

use knot_processes::{Activity, ProcessRecord, ProcessTable};
use uuid::Uuid;

use super::{ProcessSection, Showing, descendants_for, observed_roots, sample_roots};

fn record(pid: u32, ppid: u32, command: &str) -> ProcessRecord {
    ProcessRecord { pid,
                    ppid,
                    pgid: pid,
                    tpgid: 0,
                    elapsed: Duration::from_secs(u64::from(pid)),
                    command: command.to_owned() }
}

/// Two agents, each with its own root and one child, plus an unrelated
/// process that belongs to neither.
fn two_rooted_table() -> ProcessTable {
    ProcessTable::from_records([record(100, 1, "shell one"),
                                record(101, 100, "npm run dev"),
                                record(200, 1, "adapter two"),
                                record(201, 200, "rg --json"),
                                record(300, 1, "somebody else")])
}

fn section(expanded: bool) -> ProcessSection {
    let mut section = ProcessSection::default();
    if expanded {
        section.toggle();
    }
    section
}

#[test]
fn a_section_starts_collapsed_and_toggles() {
    let mut section = ProcessSection::default();

    assert!(!section.is_expanded(), "the spec starts it collapsed");

    assert!(section.toggle());
    assert!(section.is_expanded());

    assert!(!section.toggle());
    assert!(!section.is_expanded());
}

#[test]
fn a_section_with_no_sample_yet_reports_an_unknown_count() {
    let section = ProcessSection::default();

    assert_eq!(section.background_count(), None, "unknown, not zero");
    assert_eq!(section.processes(), None);
}

#[test]
fn publishing_a_sample_counts_only_the_background_ones() {
    let mut section = ProcessSection::default();

    section.publish(vec![descendant(10, Activity::Background),
                         descendant(11, Activity::Background),
                         descendant(12, Activity::Foreground)]);

    assert_eq!(section.background_count(), Some(2));
    assert_eq!(section.processes().map(<[_]>::len), Some(3));
}

#[test]
fn an_empty_sample_counts_zero_rather_than_unknown() {
    let mut section = ProcessSection::default();

    section.publish(Vec::new());

    assert_eq!(section.background_count(), Some(0));
}

#[test]
fn a_failed_sample_keeps_the_last_successful_list() {
    let mut section = ProcessSection::default();
    section.publish(vec![descendant(10, Activity::Background)]);

    section.fail("ps is missing".to_owned());

    assert_eq!(section.failure(), Some("ps is missing"));
    assert_eq!(section.processes().map(<[_]>::len),
               Some(1),
               "the rows survive the failure");
}

#[test]
fn a_later_success_clears_the_failure_silently() {
    let mut section = ProcessSection::default();
    section.publish(vec![descendant(10, Activity::Background)]);
    section.fail("ps is missing".to_owned());

    section.publish(vec![descendant(11, Activity::Background)]);

    assert_eq!(section.failure(), None);
}

#[test]
fn clearing_forgets_the_sample_without_closing_the_section() {
    let mut section = section(true);
    section.publish(vec![descendant(10, Activity::Background)]);
    section.fail("stale".to_owned());
    section.mark_terminating(10);

    section.clear();

    assert!(section.is_expanded(),
            "the user's disclosure is theirs, not the sampler's");
    assert_eq!(section.background_count(), None);
    assert_eq!(section.failure(), None);
    assert!(!section.is_terminating(10));
}

#[test]
fn a_terminating_row_stays_marked_until_the_process_leaves_the_sample() {
    let mut section = ProcessSection::default();
    section.publish(vec![descendant(10, Activity::Background),
                         descendant(11, Activity::Background)]);

    section.mark_terminating(10);
    assert!(section.is_terminating(10));

    // Still there: the grace period has not elapsed.
    section.publish(vec![descendant(10, Activity::Background),
                         descendant(11, Activity::Background)]);
    assert!(section.is_terminating(10));

    // Gone: the process exited, so the mark goes with it.
    section.publish(vec![descendant(11, Activity::Background)]);
    assert!(!section.is_terminating(10));
}

#[test]
fn one_pass_populates_every_observed_root() {
    let one = Uuid::new_v4();
    let two = Uuid::new_v4();
    let roots = BTreeMap::from([(one, 100), (two, 200)]);

    let descendants = descendants_for(&two_rooted_table(), &roots);

    assert_eq!(descendants[&one].iter().map(|p| p.pid).collect::<Vec<_>>(),
               vec![101]);
    assert_eq!(descendants[&two].iter().map(|p| p.pid).collect::<Vec<_>>(),
               vec![201]);
}

#[test]
fn two_observed_roots_cost_exactly_one_read_of_the_table() {
    let one = Uuid::new_v4();
    let two = Uuid::new_v4();
    let roots = BTreeMap::from([(one, 100), (two, 200)]);
    let reads = Cell::new(0);

    let descendants = sample_roots(&roots, || {
                          reads.set(reads.get() + 1);
                          Ok(two_rooted_table())
                      }).unwrap();

    assert_eq!(reads.get(), 1, "one `ps` serves the whole window");
    assert_eq!(descendants.len(), 2);
    assert!(descendants.contains_key(&one));
    assert!(descendants.contains_key(&two));
}

#[test]
fn nothing_observed_reads_nothing_at_all() {
    let reads = Cell::new(0);

    let descendants = sample_roots(&BTreeMap::new(), || {
                          reads.set(reads.get() + 1);
                          Ok(two_rooted_table())
                      }).unwrap();

    assert_eq!(reads.get(), 0, "no observed root, no subprocess");
    assert!(descendants.is_empty());
}

#[test]
fn a_failed_read_fails_the_pass() {
    let roots = BTreeMap::from([(Uuid::new_v4(), 100)]);

    let failed = sample_roots(&roots, || {
        Err(knot_processes::ProcessError::Parse("nope".to_owned()))
    });

    assert!(failed.is_err());
}

#[test]
fn an_expanded_section_on_the_shown_agent_is_observed() {
    let agent = Uuid::new_v4();
    let sections = BTreeMap::from([(agent, section(true))]);

    let observed = observed_roots(&sections, Showing { agent: Some(agent) }, |_| Some(100));

    assert_eq!(observed, BTreeMap::from([(agent, 100)]));
}

#[test]
fn collapsing_the_section_stops_the_sampling() {
    let agent = Uuid::new_v4();
    let sections = BTreeMap::from([(agent, section(false))]);

    let observed = observed_roots(&sections, Showing { agent: Some(agent) }, |_| Some(100));

    assert!(observed.is_empty());
}

#[test]
fn an_agent_that_stopped_is_no_longer_observed() {
    let agent = Uuid::new_v4();
    let sections = BTreeMap::from([(agent, section(true))]);

    let observed = observed_roots(&sections, Showing { agent: Some(agent) }, |_| None);

    assert!(observed.is_empty(), "no session root, nothing to sample");
}

#[test]
fn a_pane_that_is_no_longer_shown_stops_the_sampling() {
    let agent = Uuid::new_v4();
    let other = Uuid::new_v4();
    let sections = BTreeMap::from([(agent, section(true))]);

    // Another agent selected.
    assert!(observed_roots(&sections, Showing { agent: Some(other) }, |_| Some(100)).is_empty());

    // A takeover view -- the dashboard, the pull requests list -- showing no
    // agent pane at all.
    assert!(observed_roots(&sections, Showing { agent: None }, |_| Some(100)).is_empty());
}

#[test]
fn an_agent_with_no_section_at_all_is_not_observed() {
    let sections = BTreeMap::new();

    let observed = observed_roots(&sections, Showing { agent: Some(Uuid::new_v4()), }, |_| {
        Some(100)
    });

    assert!(observed.is_empty());
}

/// The one rule this module exists to keep: a render never enumerates
/// processes.
///
/// Asserted over the sources rather than trusted, because the failure is
/// invisible at runtime - the app stays correct and merely runs `ps` at
/// keystroke rate, which is exactly how `git diff --numstat` reached the
/// render path three times.
#[test]
fn no_render_module_reaches_for_the_process_table() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let render_paths = ["workspace_window/render", "panel_view", "terminal_view.rs"];

    let mut offenders = Vec::new();
    for path in render_paths {
        visit_rust_files(&src.join(path), &mut |file| {
            let text = std::fs::read_to_string(file).expect("read a render source");
            if text.contains("knot_processes") {
                offenders.push(file.display().to_string());
            }
        });
    }

    assert!(offenders.is_empty(),
            "render code must read the published snapshot, never sample: {offenders:?}");
}

fn visit_rust_files(path: &std::path::Path, found: &mut impl FnMut(&std::path::Path)) {
    if path.is_file() {
        found(path);
        return;
    }

    let Ok(entries) = std::fs::read_dir(path)
    else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() || path.extension().is_some_and(|extension| extension == "rs") {
            visit_rust_files(&path, found);
        }
    }
}

fn descendant(pid: u32, activity: Activity) -> knot_processes::DescendantProcess {
    knot_processes::DescendantProcess { pid,
                                        ppid: 1,
                                        command: format!("process {pid}"),
                                        elapsed: Duration::from_secs(u64::from(pid)),
                                        activity }
}
