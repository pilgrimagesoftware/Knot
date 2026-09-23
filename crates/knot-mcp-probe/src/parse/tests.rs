//! The fixture is the contract. `claude mcp list` has no `--json`, so the
//! only thing standing between a format change and a silently wrong section
//! is this file plus the degradation rules it checks.

use super::ListFormat;
use crate::error::ProbeError;
use crate::server::{ServerRow, Target};
use crate::state::ServerState;

/// Captured from real `claude mcp list` output, with hosts changed and the
/// inline program shortened. Every shape in it was observed: names holding
/// spaces, colons and parentheses; a stdio command containing ` - `; the
/// `(HTTP)` marker present on some entries and absent on others; and all
/// five reported states.
const CLAUDE_LISTING: &str = include_str!("fixtures/claude_mcp_list.txt");

fn rows() -> Vec<ServerRow> {
    ListFormat::ClaudeCode.read("claude", CLAUDE_LISTING)
                          .expect("the fixture is a listing")
}

fn row(name: &str) -> ServerRow {
    rows().into_iter()
          .find(|row| row.name == name)
          .unwrap_or_else(|| {
              panic!("no row named {name:?} in {:?}",
                     rows().iter().map(|r| r.name.clone()).collect::<Vec<_>>())
          })
}

#[test]
fn the_progress_header_is_not_a_server() {
    let names: Vec<_> = rows().iter().map(|row| row.name.clone()).collect();

    assert!(!names.iter().any(|name| name.contains("Checking")),
            "the health-check header became a row: {names:?}");
    assert_eq!(rows().len(),
               11,
               "one row per entry in the fixture, no more");
}

/// Names are not identifiers. The real listing holds spaces, dots,
/// parentheses and colons, and the colon is the one that matters: splitting
/// on the last `: ` instead of the first would name this server "github".
#[test]
fn a_name_may_hold_spaces_dots_parentheses_and_colons() {
    assert_eq!(row("claude.ai Claude Docs").state, ServerState::Connected);
    assert_eq!(row("claude.ai Asana (2)").state, ServerState::Disabled);
    assert_eq!(row("plugin:kochava:github").state, ServerState::Connected);
}

/// The entry that dictates the whole parsing strategy. Its command contains
/// ` - ` three times, so any left-anchored split cuts it apart and any split
/// that does not check what follows the separator lands inside the program.
#[test]
fn a_stdio_command_containing_the_separator_survives_whole() {
    let row = row("plugin:claude-mem:mcp-search");

    assert_eq!(row.state, ServerState::Connected);

    let Target::Stdio { command } = &row.target
    else {
        panic!("an inline node program is not an HTTP target: {:?}",
               row.target)
    };

    assert!(command.starts_with("node -e"),
            "command began {command:.40?}");
    assert!(command.contains("/* a - b - c */"),
            "the separator inside the command was treated as the state split");
    assert!(command.ends_with("{stdio:'inherit'});"),
            "the command lost its tail: {command:.80?}");
    assert_eq!(row.short_label(), "node");
}

#[test]
fn transports_are_read_from_the_target_not_the_marker() {
    assert!(matches!(row("knot").target, Target::Http { .. }));
    assert!(matches!(row("claude.ai Claude Docs").target, Target::Http { .. }),
            "an entry without a (HTTP) marker is still HTTP when its target is a URL");
    assert!(matches!(row("plugin:grafana-mcp:grafana").target,
                     Target::Stdio { .. }));
}

#[test]
fn the_transport_marker_is_stripped_from_the_url() {
    let Target::Http { url } = &row("knot").target
    else {
        panic!("expected HTTP")
    };

    assert_eq!(url, "http://127.0.0.1:8767/mcp",
               "the (HTTP) marker stayed on the URL");
}

/// The four non-connected states the real tooling reports. Collapsing any of
/// them into the others is the mistake the vocabulary exists to prevent.
#[test]
fn every_reported_state_maps_to_its_own_value() {
    assert_eq!(row("claude.ai Asana").state, ServerState::Connected);
    assert_eq!(row("tolaria").state, ServerState::Disabled);
    assert_eq!(row("sentry").state, ServerState::PendingApproval);
    assert_eq!(row("aws-mcp").state, ServerState::Failed);
    assert_eq!(row("linear").state, ServerState::NeedsAuthentication);
}

/// Deliberate states are not failures. A user who turned a server off is not
/// being asked to fix anything.
#[test]
fn disabled_and_pending_do_not_need_attention() {
    assert!(!row("tolaria").state.needs_attention());
    assert!(!row("sentry").state.needs_attention());

    assert!(row("aws-mcp").state.needs_attention());
    assert!(row("linear").state.needs_attention());
}

/// A server needing authentication is marked with the same ✘ as a connection
/// failure. Reading the glyph alone would send the user to repair something
/// that is merely locked.
#[test]
fn needing_authentication_is_not_read_as_a_failure() {
    let row = row("linear");

    assert_eq!(row.state, ServerState::NeedsAuthentication);
    assert_ne!(row.state, ServerState::Failed);
}

#[test]
fn a_failure_keeps_the_reason_the_cli_gave() {
    let row = row("aws-mcp");

    assert_eq!(row.state, ServerState::Failed);
    assert_eq!(row.detail.as_deref(),
               Some("-32602: Invalid request parameters"));
}

#[test]
fn a_connected_server_carries_no_reason() {
    assert_eq!(row("knot").detail, None);
}

/// Per-line degradation. A format change that alters one entry's state must
/// not shorten the list: the server is still there and Knot can still name
/// it, which is strictly more useful than dropping it.
#[test]
fn an_unreadable_entry_becomes_an_unknown_row_rather_than_vanishing() {
    let listing = "good: https://a.example/mcp (HTTP) - ✔ Connected\n\
                   odd: https://b.example/mcp (HTTP) - ◈ Quantum superposition\n";

    let rows = ListFormat::ClaudeCode.read("claude", listing)
                                     .expect("one line still parses");

    assert_eq!(rows.len(), 2, "the unreadable entry was dropped");
    assert_eq!(rows[1].name, "odd");
    assert_eq!(rows[1].state, ServerState::Unknown);
    assert_eq!(rows[1].detail.as_deref(),
               Some("◈ Quantum superposition"),
               "what it said is the only thing worth showing when we cannot read it");
}

/// An unrecognized glyph must never land on the optimistic answer. Reporting
/// a server as connected when we have no idea is the one wrong answer that
/// hides the problem instead of surfacing it.
#[test]
fn an_unreadable_entry_is_never_reported_as_connected() {
    let listing = "good: https://a.example/mcp - ✔ Connected\n\
                   odd: https://b.example/mcp - ??? something new\n";

    let rows = ListFormat::ClaudeCode.read("claude", listing).unwrap();

    assert_ne!(rows[1].state, ServerState::Connected);
    assert_eq!(rows[1].state, ServerState::Unknown);
}

/// Wholesale degradation. Output with no entry in it at all is a format
/// change or an error, and calling it "no servers configured" would be a
/// confident wrong answer.
#[test]
fn foreign_output_is_an_error_and_not_an_empty_list() {
    let usage = "Usage: claude mcp list [options]\n\nOptions:\n  -h, --help  Display help\n";

    let err = ListFormat::ClaudeCode.read("claude", usage)
                                    .expect_err("a usage message is not a listing");

    assert!(matches!(&err, ProbeError::Unrecognized { program, .. } if program == "claude"),
            "got {err:?}");
}

/// The quoted line has to be the one that was actually unreadable. The
/// listing opens with a progress header, and naming that would point at the
/// one line that was never the problem.
#[test]
fn the_error_quotes_an_entry_rather_than_the_progress_header() {
    let moved = "Checking MCP server health…\n\ngithub: https://a.example/mcp :: ONLINE\n";

    let err = ListFormat::ClaudeCode.read("claude", moved)
                                    .expect_err("the format moved");

    match err {
        ProbeError::Unrecognized { first_line, .. } => {
            assert!(first_line.starts_with("github:"), "quoted {first_line:?}");
        }
        other => panic!("got {other:?}"),
    }
}

#[test]
fn empty_output_is_an_error_rather_than_an_empty_list() {
    assert!(ListFormat::ClaudeCode.read("claude", "").is_err());
    assert!(ListFormat::ClaudeCode.read("claude", "   \n\n  ").is_err());
}
