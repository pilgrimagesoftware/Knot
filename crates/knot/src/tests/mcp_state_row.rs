//! The MCP tab's read-only server-state row.
//!
//! Contract: the `settings-ui` delta of
//! `openspec/changes/supervise-mcp-server/`. These assert which state the
//! row reports and that it never reports data belonging to another one;
//! the copy itself is the catalog's, so nothing here asserts English.

use std::net::SocketAddr;
use std::time::Duration;

use knot_mcp::ServerState;

use crate::settings_window::SettingsWindow;

fn addr() -> SocketAddr {
    ([127, 0, 0, 1], 8767).into()
}

fn every_state() -> Vec<ServerState> {
    vec![ServerState::Disabled,
         ServerState::Starting,
         ServerState::Running { addr: addr() },
         ServerState::Retrying { attempt:    2,
                                 next_delay: Duration::from_secs(1),
                                 error:      "address in use".to_string(), },
         ServerState::Stopped]
}

#[test]
fn every_state_has_its_own_catalog_entry() {
    for key in ["settings.mcp.status",
                "settings.mcp.status_disabled",
                "settings.mcp.status_starting",
                "settings.mcp.status_running",
                "settings.mcp.status_retrying",
                "settings.mcp.status_stopped"]
    {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} is missing from the catalog");
    }
}

#[test]
fn every_state_renders_distinct_text() {
    let mut texts: Vec<String> = every_state().iter()
                                              .map(SettingsWindow::mcp_state_text)
                                              .collect();
    for text in &texts {
        assert!(!text.is_empty(),
                "a state with no text tells the user nothing");
        assert!(!text.contains("settings.mcp."),
                "the key leaked through: {text}");
    }
    texts.sort();
    texts.dedup();
    assert_eq!(texts.len(), 5, "two states must not read the same");
}

#[test]
fn running_shows_its_address_and_nothing_of_another_state() {
    let text = SettingsWindow::mcp_state_text(&ServerState::Running { addr: addr() });

    assert!(text.contains(&addr().to_string()),
            "the bound address is what running has to say");
    assert!(!text.contains("%{address}"),
            "the placeholder was substituted");
    assert!(!text.contains("address in use"),
            "a running server has no error to report");
}

#[test]
fn retrying_shows_its_attempt_and_error_and_no_address() {
    let text =
        SettingsWindow::mcp_state_text(&ServerState::Retrying { attempt:    4,
                                                                next_delay: Duration::from_secs(8),
                                                                error:
                                                                    "address in use".to_string(), });

    assert!(text.contains('4'),
            "the attempt number is part of what retrying reports");
    assert!(text.contains("address in use"),
            "so is the last attempt's error");
    assert!(!text.contains("%{attempt}") && !text.contains("%{error}"),
            "both placeholders were substituted: {text}");
    assert!(!text.contains(&addr().to_string()),
            "a retrying server is bound to nothing");
}

#[test]
fn the_payload_free_states_report_only_themselves() {
    for state in [ServerState::Disabled,
                  ServerState::Starting,
                  ServerState::Stopped]
    {
        let text = SettingsWindow::mcp_state_text(&state);
        assert!(!text.contains(&addr().to_string()),
                "{state:?} showed an address");
        assert!(!text.contains("address in use"),
                "{state:?} showed an error");
    }
}

#[test]
fn an_unchanged_state_asks_for_no_repaint() {
    for state in every_state() {
        let mut last = state.clone();
        assert!(!SettingsWindow::mcp_state_changed(&mut last, state.clone()),
                "{state:?} repainted without changing");
        assert_eq!(last, state,
                   "an unchanged tick leaves the remembered state alone");
    }
}

#[test]
fn a_changed_state_asks_for_a_repaint_once() {
    let mut last = ServerState::Starting;
    let running = ServerState::Running { addr: addr() };

    assert!(SettingsWindow::mcp_state_changed(&mut last, running.clone()));
    assert_eq!(last, running, "the row now remembers what it drew");
    assert!(!SettingsWindow::mcp_state_changed(&mut last, running),
            "the next tick of the same state is quiet");
}

#[test]
fn a_changed_payload_within_one_variant_asks_for_a_repaint() {
    let mut last = ServerState::Retrying { attempt:    1,
                                           next_delay: Duration::from_millis(500),
                                           error:      "address in use".to_string(), };
    let later = ServerState::Retrying { attempt:    2,
                                        next_delay: Duration::from_secs(1),
                                        error:      "address in use".to_string(), };

    assert!(SettingsWindow::mcp_state_changed(&mut last, later),
            "a further attempt is a change the row shows");
}
