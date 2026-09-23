//! Captured from real `gemini mcp list` output, with hosts changed.

use crate::parse::ListFormat;
use crate::server::{ServerRow, Target};
use crate::state::ServerState;

const GEMINI_LISTING: &str = include_str!("../fixtures/gemini_mcp_list.txt");

fn rows() -> Vec<ServerRow> {
    ListFormat::Gemini.read("gemini", GEMINI_LISTING)
                      .expect("the fixture is a listing")
}

fn row(name: &str) -> ServerRow {
    rows().into_iter()
          .find(|row| row.name == name)
          .unwrap_or_else(|| panic!("no row named {name:?}"))
}

/// The line that made an entry require *both* separators. It has the
/// `name: value` shape, and on the earlier rule it became a server called
/// "Warning" sitting at the top of the user's list.
#[test]
fn the_untrusted_folder_warning_is_not_a_server() {
    let names: Vec<_> = rows().iter().map(|row| row.name.clone()).collect();

    assert!(!names.iter().any(|name| name.starts_with("Warning")),
            "the warning became a row: {names:?}");
    assert!(!names.iter().any(|name| name.starts_with("User-level")),
            "got {names:?}");
    assert_eq!(rows().len(), 4, "one row per entry, no more");
}

/// Gemini puts its status glyph at the start of the line, where Claude Code
/// puts one after the separator. Leaving it on would name this server
/// "○ tolaria".
#[test]
fn a_leading_status_glyph_is_not_part_of_the_name() {
    assert_eq!(row("tolaria").state, ServerState::Disabled);
    assert_eq!(row("knot").state, ServerState::Connected);
    assert_eq!(row("linear").state, ServerState::Failed);
}

/// Gemini names its states in words with no glyph at all, so the wording
/// fallback is the only thing reading them.
#[test]
fn states_are_read_from_the_wording_alone() {
    assert_eq!(row("aws-mcp").state, ServerState::Disabled);
    assert_eq!(row("knot").state, ServerState::Connected);
    assert_eq!(row("linear").state, ServerState::Failed);

    assert!(!row("tolaria").state.needs_attention(),
            "a disabled server is a choice, not a problem");
    assert!(row("linear").state.needs_attention());
}

#[test]
fn the_stdio_transport_marker_is_stripped_from_the_command() {
    let Target::Stdio { command } = &row("tolaria").target
    else {
        panic!("expected stdio, got {:?}", row("tolaria").target)
    };

    assert!(command.ends_with("index.js"),
            "the (stdio) marker stayed on: {command:?}");
    assert_eq!(row("tolaria").short_label(), "node");
}

/// A command holding a URL is still a command. Reading the transport from
/// the marker rather than the target would call this one HTTP.
#[test]
fn a_stdio_command_containing_a_url_is_not_an_http_target() {
    assert!(matches!(row("aws-mcp").target, Target::Stdio { .. }));
    assert_eq!(row("aws-mcp").short_label(), "uvx");
}

#[test]
fn an_http_entry_keeps_its_url() {
    let Target::Http { url } = &row("knot").target
    else {
        panic!("expected HTTP")
    };

    assert_eq!(url, "http://127.0.0.1:8767/mcp");
}

/// Knot's own server registered by hand in Gemini's configuration has to be
/// recognizable, or the section shows it twice.
#[test]
fn knots_own_endpoint_is_recognizable_in_a_gemini_listing() {
    let Target::Http { url } = &row("knot").target
    else {
        panic!("expected HTTP")
    };

    assert!(crate::same_endpoint(url, "http://localhost:8767/mcp/"));
}
