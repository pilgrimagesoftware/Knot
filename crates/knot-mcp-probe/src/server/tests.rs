use super::{ServerRow, Target};
use crate::consts::MAX_LABEL_CHARS;
use crate::state::ServerState;

/// The entry that made the bound a requirement rather than a nicety: a real
/// `claude mcp list` row whose command was a ~1.5 KB inline program.
fn huge_node_command() -> String {
    let body = "const f=require('fs'),p=require('path');".repeat(40);

    format!("node -e {body}")
}

#[test]
fn a_long_stdio_command_labels_as_its_program() {
    let command = huge_node_command();
    assert!(command.len() > 1_000, "the fixture must actually be long");

    let target = Target::Stdio { command: command.clone(), };

    assert_eq!(target.short_label(), "node");
    assert!(target.short_label().chars().count() <= MAX_LABEL_CHARS);
    assert_eq!(target.full(), command, "the whole command stays reachable");
}

#[test]
fn a_program_path_labels_as_its_basename() {
    let target =
        Target::Stdio { command: "/opt/homebrew/bin/uvx --from mcp-server run".to_owned(), };

    assert_eq!(target.short_label(), "uvx");
}

#[test]
fn an_http_target_labels_as_its_host_and_port() {
    let target = Target::Http { url: "http://127.0.0.1:8767/mcp".to_owned(), };

    assert_eq!(target.short_label(), "127.0.0.1:8767");
}

/// Userinfo in a URL is a credential. A row that drew one would leak it to
/// anyone looking at the screen.
#[test]
fn credentials_in_a_url_never_reach_the_label() {
    let target = Target::Http { url: "https://user:s3cret@mcp.example.com/sse".to_owned(), };

    let label = target.short_label();
    assert_eq!(label, "mcp.example.com");
    assert!(!label.contains("s3cret"));
    assert!(!label.contains("user"));
}

/// A single token longer than the bound has no basename to fall back to, so
/// truncation is the only thing standing between it and the row height.
#[test]
fn a_single_enormous_token_is_truncated_and_marked() {
    let command = "x".repeat(500);
    let target = Target::Stdio { command };

    let label = target.short_label();
    assert_eq!(label.chars().count(), MAX_LABEL_CHARS);
    assert!(label.ends_with('…'), "a truncated label says that it was");
}

/// The output is another program's; "not a URL" has to be an ordinary case
/// rather than a panic or an empty row.
#[test]
fn malformed_targets_still_produce_a_bounded_label() {
    for text in ["", "not a url at all", "://", "http://"] {
        let target = Target::Http { url: text.to_owned(), };
        assert!(target.short_label().chars().count() <= MAX_LABEL_CHARS,
                "{text:?} produced an unbounded label");
    }
}

#[test]
fn transport_tokens_are_stable() {
    assert_eq!(Target::Http { url: "http://x".to_owned(), }.token(), "http");
    assert_eq!(Target::Stdio { command: "node".to_owned(), }.token(),
               "stdio");
}

#[test]
fn a_blank_detail_is_no_detail() {
    let row = ServerRow::new("github",
                             Target::Http { url: "https://api.example".to_owned(), },
                             ServerState::Failed).with_detail("   ");

    assert_eq!(row.detail, None,
               "whitespace is not a reason to show a reason");
}

#[test]
fn a_detail_is_kept_beside_the_state() {
    let row = ServerRow::new("github",
                             Target::Http { url: "https://api.example".to_owned(), },
                             ServerState::Failed).with_detail("-32602: Invalid request parameters");

    assert_eq!(row.state, ServerState::Failed);
    assert_eq!(row.detail.as_deref(),
               Some("-32602: Invalid request parameters"));
}
