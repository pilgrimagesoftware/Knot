//! Samples the real process table. See
//! `openspec/specs/agent-processes/spec.md` -- "Descendant processes are
//! enumerated transitively".

use knot_processes::sample;

#[test]
fn the_table_contains_this_test_binary() {
    let table = sample().unwrap();
    let me = std::process::id();

    let record = table.get(me)
                      .expect("this test binary is in the process table");

    assert_eq!(record.pid, me);
    assert!(!record.command.is_empty(), "a process has a command line");
    assert_ne!(record.ppid, 0, "a test binary has a parent");
}

#[test]
fn the_table_covers_the_whole_machine() {
    let table = sample().unwrap();

    assert!(table.len() > 1,
            "read {} processes, expected the whole table",
            table.len());
}

#[test]
fn a_spawned_child_appears_as_a_descendant_of_this_process() {
    let mut child = std::process::Command::new("sleep").arg("30")
                                                       .spawn()
                                                       .expect("spawn sleep");
    let child_pid = child.id();

    let table = sample().unwrap();
    let descendants = table.descendants(std::process::id());

    let found = descendants.iter().find(|process| process.pid == child_pid);

    let _ = child.kill();
    let _ = child.wait();

    let found = found.expect("the spawned child descends from this test process");
    assert!(found.command.contains("sleep"));
}
