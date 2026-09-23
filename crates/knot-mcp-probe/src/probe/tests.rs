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

/// An agent that genuinely has none is the other answer, and the CLI says so
/// in words rather than printing an empty listing. Reading that phrase is
/// what separates "found none" from "could not read this".
#[test]
fn a_cli_saying_there_are_none_is_found_none_and_not_an_error() {
    let runner = StubRunner::ok("No MCP servers configured. Use `claude mcp add` to add one.\n");

    let inventory = probe(&runner, &plan()).expect("an answer, not a failure");

    assert!(inventory.found_none());
    assert!(!inventory.is_unprobeable(), "we asked, and got an answer");
    assert!(inventory.taken_at().is_some());
}

/// Output that says neither "here are the servers" nor "there are none" is
/// the format having moved, and must not be reported as an empty list.
#[test]
fn output_that_says_nothing_recognizable_is_an_error() {
    let runner = StubRunner::ok("Usage: claude mcp list [options]\n");

    let err = probe(&runner, &plan()).expect_err("a usage message is not an answer");

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

/// The coverage this crate owes the roster. A type that can be listed but has
/// no reader here produces `Unsupported` at runtime while the roster claims
/// otherwise - the silent drift the split between the two tables invites.
#[test]
fn every_listable_type_has_a_reader_and_the_reverse() {
    for agent_type in knot_core::agent_type::ALL {
        let listable = knot_core::agent_type::mcp_list_command(agent_type.id).is_some();
        let readable = ListFormat::for_agent_type(agent_type.id).is_some();

        assert_eq!(listable, readable,
                   "{} has a list command ({listable}) but a reader ({readable})",
                   agent_type.id);
    }
}

#[test]
fn a_plan_is_built_from_the_agent_type() {
    let plan = super::plan_for("claude", Path::new("/projects/api"), vec![], None);

    let ProbePlan::Run { command, format } = plan
    else {
        panic!("claude is listable")
    };

    assert_eq!(command.label(), "claude mcp list");
    assert_eq!(command.cwd, Path::new("/projects/api"));
    assert_eq!(format, ListFormat::ClaudeCode);
}

/// A user who points Knot at a particular build means that build. A probe
/// that ran whatever was first on `PATH` would read another installation's
/// configuration and report servers the agent does not have.
#[test]
fn a_configured_command_replaces_the_default_program() {
    let plan = super::plan_for("claude",
                               Path::new("/projects/api"),
                               vec![],
                               Some("/opt/custom/claude --resume"));

    let ProbePlan::Run { command, .. } = plan
    else {
        panic!("claude is listable")
    };

    assert_eq!(command.program, "/opt/custom/claude");
    assert_eq!(command.args,
               vec!["mcp".to_owned(), "list".to_owned()],
               "the setting's launch flags are not arguments to `mcp list`");
}

/// Not "no servers" - Knot has no way to ask. A shell runs no MCP client at
/// all, and an unrecognized type is a working agent Knot knows nothing about.
#[test]
fn a_type_with_no_reader_yields_an_unsupported_plan() {
    for id in ["shell", "codex", "copilot", "nothing-by-that-name"] {
        assert!(matches!(super::plan_for(id, Path::new("/projects/api"), vec![], None),
                         ProbePlan::Unsupported),
                "{id} should not be probeable");
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
