//! Three of these four rows are captured from real `codex mcp list --json`
//! output (names and paths changed): an enabled HTTP server, a disabled
//! stdio one, and an enabled server whose transport declares OAuth. The
//! fourth - a disabled stdio server carrying a `disabled_reason` - was not
//! observed on the machine this was written on and is here only so the
//! field, which the schema does carry, has a test at all.

use super::parse;
use crate::server::Target;
use crate::state::ServerState;

const CODEX_LISTING: &str = include_str!("../fixtures/codex_mcp_list.json");

fn rows() -> Vec<crate::server::ServerRow> {
    parse(CODEX_LISTING).expect("the fixture is a listing")
}

fn row(name: &str) -> crate::server::ServerRow {
    rows().into_iter()
          .find(|row| row.name == name)
          .unwrap_or_else(|| panic!("no row named {name:?}"))
}

#[test]
fn one_row_per_entry() {
    assert_eq!(rows().len(), 4);
}

/// Codex's listing is configuration, not a live probe - it never saw the
/// server answer, so an enabled row cannot honestly claim `Connected`.
#[test]
fn an_enabled_server_is_unknown_not_connected() {
    assert_eq!(row("akeyless").state, ServerState::Unknown);
    assert_eq!(row("sentry").state, ServerState::Unknown);
}

/// `auth_status: "o_auth"` names the transport's auth mechanism, not
/// whether the user has completed it - reading it as `NeedsAuthentication`
/// would be a guess this crate has no basis for.
#[test]
fn an_oauth_transport_is_still_unknown_not_needing_authentication() {
    assert_eq!(row("sentry").state, ServerState::Unknown);
}

#[test]
fn a_disabled_server_is_disabled() {
    assert_eq!(row("codex_app").state, ServerState::Disabled);
    assert_eq!(row("legacy-tool").state, ServerState::Disabled);
}

#[test]
fn a_disabled_reason_becomes_the_row_detail() {
    assert_eq!(row("codex_app").detail, None);
    assert_eq!(row("legacy-tool").detail.as_deref(),
               Some("removed by the user"));
}

#[test]
fn an_http_transport_reads_its_url() {
    let Target::Http { url } = &row("akeyless").target
    else {
        panic!("expected HTTP, got {:?}", row("akeyless").target)
    };

    assert_eq!(url, "http://127.0.0.1:8086/mcp");
}

#[test]
fn a_stdio_transport_keeps_its_command_and_args_together() {
    let Target::Stdio { command } = &row("legacy-tool").target
    else {
        panic!("expected stdio, got {:?}", row("legacy-tool").target)
    };

    assert_eq!(command, "node /opt/legacy-tool/index.js");
}

#[test]
fn foreign_output_is_not_a_listing() {
    assert!(parse("Usage: codex mcp list [OPTIONS]\n").is_none());
    assert!(parse("").is_none());
}
