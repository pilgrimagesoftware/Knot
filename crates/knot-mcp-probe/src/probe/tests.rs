use std::path::Path;

use super::{ProbePlan, probe};
use crate::error::ProbeError;
use crate::parse::ListFormat;
use crate::runner::ProbeCommand;
use crate::runner::stub::StubRunner;
use crate::state::ServerState;

fn plan() -> ProbePlan {
    ProbePlan::Run { command: ProbeCommand::new("claude",
                                                vec!["mcp".to_owned(), "list".to_owned()],
                                                Path::new("/projects/api")),
                     format:  ListFormat::ClaudeCode, }
}

#[test]
fn a_listing_becomes_a_stamped_inventory() {
    let runner = StubRunner::ok("github: https://a.example/mcp (HTTP) - ✔ Connected\n\
                                 sentry: https://b.example/mcp (HTTP) - ✘ Failed to connect — down\n");

    let inventory = probe(&runner, &plan()).expect("a listing");

    assert_eq!(inventory.rows().len(), 2);
    assert_eq!(inventory.attention_count(), 1);
    assert!(inventory.taken_at().is_some(),
            "rows must carry when they were taken");
    assert!(!inventory.is_unprobeable());
}

/// The probe runs the agent's command in the agent's directory. Anything else
/// resolves a different project's configuration.
#[test]
fn the_agents_command_and_directory_are_what_is_run() {
    let runner = StubRunner::ok("github: https://a.example/mcp - ✔ Connected\n");

    probe(&runner, &plan()).unwrap();

    let calls = runner.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].label(), "claude mcp list");
    assert_eq!(calls[0].cwd, Path::new("/projects/api"));
}

/// An agent type with no list command must not run anything, and must not
/// come back looking like an agent with no servers.
#[test]
fn an_unsupported_type_runs_nothing_and_is_not_an_empty_list() {
    let runner = StubRunner::ok("should not be asked");

    let inventory = probe(&runner, &ProbePlan::Unsupported).expect("not an error");

    assert!(inventory.is_unprobeable());
    assert!(!inventory.found_none(), "unprobeable is not 'found none'");
    assert!(runner.calls().is_empty(), "nothing should have been run");
}

/// An agent that genuinely has none is the other answer, and it is an `Ok`
/// empty list rather than an error.
#[test]
fn a_listing_with_no_entries_is_an_error_not_a_silent_empty() {
    // `claude mcp list` with nothing configured prints a message, not an
    // empty listing, so there is no entry to anchor on. Reporting that as
    // "no servers" would be guessing; the section says it could not read it.
    let runner = StubRunner::ok("No MCP servers configured. Use `claude mcp add` to add one.\n");

    let err = probe(&runner, &plan()).expect_err("nothing anchored the parse");

    assert!(matches!(err, ProbeError::Unrecognized { .. }),
            "got {err:?}");
}

#[test]
fn a_missing_binary_surfaces_as_missing() {
    let runner = StubRunner::missing("claude");

    let err = probe(&runner, &plan()).expect_err("claude is not installed");

    assert!(matches!(&err, ProbeError::Missing { program } if program == "claude"),
            "got {err:?}");
    assert!(err.is_persistent());
}

#[test]
fn a_failing_command_surfaces_its_output() {
    let runner = StubRunner::failing("not logged in", 1);

    let err = probe(&runner, &plan()).expect_err("the command failed");

    match err {
        ProbeError::Command { output, code, .. } => {
            assert_eq!(code, 1);
            assert_eq!(output, "not logged in");
        }
        other => panic!("got {other:?}"),
    }
}

#[test]
fn unknown_states_survive_into_the_inventory() {
    let runner = StubRunner::ok("good: https://a.example/mcp - ✔ Connected\n\
                                 odd: https://b.example/mcp - ◈ new thing\n");

    let inventory = probe(&runner, &plan()).unwrap();

    assert_eq!(inventory.rows().len(), 2);
    assert!(inventory.any_in(ServerState::Unknown));
    assert_eq!(inventory.attention_count(), 0, "unknown is not an alarm");
}
