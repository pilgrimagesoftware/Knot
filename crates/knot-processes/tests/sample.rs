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

/// The limitation `design.md` documents, pinned so it is recognized as the
/// known boundary rather than rediscovered as a defect.
///
/// A process that forks twice and lets its middle parent exit is reparented
/// to `launchd`/`init`. No ancestry links it to the agent any more, so it
/// cannot appear in that agent's section. Fixing it would mean owning how
/// every agent launches every tool, which this capability deliberately does
/// not do.
#[test]
fn a_double_forked_daemon_is_reparented_away_and_cannot_be_attributed() {
    let pid_file = std::env::temp_dir().join(format!("knot-double-fork-{}", std::process::id()));
    let script = format!("(sleep 60 & echo $! > {} ) &", pid_file.display());

    let mut shell = std::process::Command::new("sh").arg("-c")
                                                    .arg(&script)
                                                    .spawn()
                                                    .expect("spawn the double-forking shell");
    shell.wait().expect("the outer shell exits at once");

    let daemon_pid = read_pid(&pid_file);
    let _ = std::fs::remove_file(&pid_file);

    // Wait for the reparenting itself rather than for a fixed duration: the
    // middle shell exits on its own schedule, and what the test is about is
    // the state after it has.
    let reparented = wait_for_reparenting(daemon_pid);

    let table = sample().unwrap();
    let mine = table.descendants(std::process::id());
    let found = mine.iter().any(|process| process.pid == daemon_pid);

    let _ = std::process::Command::new("kill").arg("-KILL")
                                              .arg(daemon_pid.to_string())
                                              .status();

    assert!(reparented,
            "the daemon was still in this process's tree after the grace period");
    assert!(!found,
            "a double-forked daemon has no ancestry back to the agent, so it cannot be listed");
}

fn read_pid(path: &std::path::Path) -> u32 {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if let Ok(text) = std::fs::read_to_string(path)
           && let Ok(pid) = text.trim().parse()
        {
            return pid;
        }

        assert!(std::time::Instant::now() < deadline,
                "the daemon never reported its pid");
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// Whether `pid`'s parent became `launchd`/`init` within the grace period.
fn wait_for_reparenting(pid: u32) -> bool {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if sample().unwrap()
                   .get(pid)
                   .is_some_and(|record| record.ppid == 1)
        {
            return true;
        }

        if std::time::Instant::now() >= deadline {
            return false;
        }

        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}
