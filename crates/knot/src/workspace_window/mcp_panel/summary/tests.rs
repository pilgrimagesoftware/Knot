//! Every branch resolves from the catalog, and the order between them holds.
//!
//! Assertions are on the key resolving and on values surviving substitution,
//! never on the English copy - a copy edit must not fail a test.

use knot_mcp_probe::{ALL_STATES, Inventory, ServerRow, ServerState, Target};

use super::{SectionStatus, state_label, summary_text};

fn row(name: &str, state: ServerState) -> ServerRow {
    ServerRow::new(name,
                   Target::Http { url: format!("https://{name}.example/mcp"), },
                   state)
}

fn status<'a>(inventory: Option<&'a Inventory>) -> SectionStatus<'a> {
    SectionStatus { is_running: true,
                    expanded: false,
                    probing: false,
                    inventory,
                    failure: None,
                    row_count: inventory.map_or(0, |i| i.rows().len() + 1) }
}

/// A key that did not resolve comes back as the key itself.
fn resolved(text: &str) {
    assert!(!text.starts_with("mcp."), "unresolved catalog key: {text}");
    assert!(!text.is_empty(), "empty summary");
}

#[test]
fn every_branch_resolves_from_the_catalog() {
    let probed = Inventory::probed(vec![row("a", ServerState::Connected)]);
    let empty = Inventory::probed(vec![]);
    let mixed = Inventory::probed(vec![row("a", ServerState::Connected),
                                       row("b", ServerState::Disabled)]);
    let attention = Inventory::probed(vec![row("a", ServerState::Failed)]);

    let cases = [SectionStatus { is_running: false,
                                 ..status(None) },
                 SectionStatus { probing: true,
                                 ..status(None) },
                 SectionStatus { failure: Some("claude is not installed"),
                                 ..status(None) },
                 status(None),
                 status(Some(&Inventory::Unprobeable)),
                 status(Some(&empty)),
                 status(Some(&probed)),
                 status(Some(&mixed)),
                 status(Some(&attention)),
                 SectionStatus { expanded: true,
                                 ..status(Some(&probed)) }];

    for case in &cases {
        resolved(&summary_text(case));
    }
}

#[test]
fn every_state_has_a_label() {
    for state in ALL_STATES {
        resolved(&state_label(*state));
    }
}

/// A stopped agent settles the question before any snapshot is consulted.
/// Leaving the last sample's names up would have the header contradicting a
/// body that says the agent is not running.
#[test]
fn not_running_outranks_everything_it_might_still_hold() {
    let attention = Inventory::probed(vec![row("github", ServerState::Failed)]);

    let text = summary_text(&SectionStatus { is_running: false,
                                             probing: true,
                                             failure: Some("boom"),
                                             ..status(Some(&attention)) });

    assert!(!text.contains("github"),
            "a stopped agent still named its servers: {text}");
    assert!(!text.contains("boom"));
}

/// A probe in flight outranks a stale answer: the header should say it is
/// looking, not report a result it is in the middle of replacing.
#[test]
fn checking_outranks_the_previous_result() {
    let attention = Inventory::probed(vec![row("github", ServerState::Failed)]);

    let text = summary_text(&SectionStatus { probing: true,
                                             ..status(Some(&attention)) });

    assert!(!text.contains("github"), "got {text}");
}

/// The failure is the news. The rows it could not refresh stay on screen
/// below, with their own timestamp - that pairing is what the requirement is
/// for, so the header must not bury the failure behind them.
#[test]
fn a_failure_is_reported_even_when_rows_survive() {
    let rows = Inventory::probed(vec![row("github", ServerState::Connected)]);

    let text = summary_text(&SectionStatus { failure: Some("claude mcp list timed out"),
                                             ..status(Some(&rows)) });

    assert!(text.contains("timed out"),
            "the failure's reason must survive substitution: {text}");
}

/// The distinction the whole change rests on. These two have no rows to draw
/// and must not say the same thing.
#[test]
fn cannot_determine_and_found_none_read_differently() {
    let cannot = summary_text(&status(Some(&Inventory::Unprobeable)));
    let none = summary_text(&status(Some(&Inventory::probed(vec![]))));

    resolved(&cannot);
    resolved(&none);
    assert_ne!(cannot, none,
               "an unprobeable agent must not read as one with no servers");
}

#[test]
fn the_collapsed_header_names_what_needs_attention() {
    let inventory = Inventory::probed(vec![row("alpha", ServerState::Connected),
                                           row("bravo", ServerState::NeedsAuthentication),
                                           row("charlie", ServerState::Failed),
                                           row("delta", ServerState::Disabled)]);

    let text = summary_text(&status(Some(&inventory)));

    assert!(text.contains("bravo"), "got {text}");
    assert!(text.contains("charlie"), "got {text}");
    assert!(!text.contains("alpha"),
            "a connected server is not asking for anything: {text}");
    assert!(!text.contains("delta"),
            "a disabled server is a choice, not a problem: {text}");
}

/// Names are capped and the remainder counts servers, not names - the
/// question behind a glance at the header is how much is wrong.
#[test]
fn the_names_are_capped_with_a_remainder() {
    let inventory =
        Inventory::probed((0..6).map(|i| row(&format!("server{i}"), ServerState::Failed))
                                .collect());

    let text = summary_text(&status(Some(&inventory)));

    assert!(text.contains("server0") && text.contains("server1") && text.contains("server2"));
    assert!(!text.contains("server5"), "the cap did not hold: {text}");
    assert!(text.contains('3'),
            "the remainder should count the three not named: {text}");
}

/// "All connected" only when that is true. A disabled or pending server is
/// not connected, and saying so to save a word would be the header lying
/// about the one thing it is for.
#[test]
fn nothing_needing_attention_is_not_the_same_as_all_connected() {
    let all_good = Inventory::probed(vec![row("a", ServerState::Connected),
                                          row("b", ServerState::Connected)]);
    let mixed = Inventory::probed(vec![row("a", ServerState::Connected),
                                       row("b", ServerState::Disabled)]);

    assert_eq!(all_good.attention_count(), 0);
    assert_eq!(mixed.attention_count(), 0);

    let all_text = summary_text(&status(Some(&all_good)));
    let mixed_text = summary_text(&status(Some(&mixed)));

    assert_ne!(all_text, mixed_text,
               "a disabled server was reported as connected");
    assert!(mixed_text.contains('2'),
            "the honest form says how many it knows about: {mixed_text}");
}

/// Expanded, the header counts the rows below it - Knot's own row included,
/// because that is one of the rows below it.
#[test]
fn expanded_counts_the_rows_that_are_drawn() {
    let inventory = Inventory::probed(vec![row("a", ServerState::Connected),
                                           row("b", ServerState::Failed)]);

    let text = summary_text(&SectionStatus { expanded: true,
                                             row_count: 3,
                                             ..status(Some(&inventory)) });

    assert!(text.contains('3'),
            "expected Knot's row to be counted too: {text}");
}

#[test]
fn one_row_reads_singular() {
    let text = summary_text(&SectionStatus { expanded: true,
                                             row_count: 1,
                                             ..status(Some(&Inventory::probed(vec![]))) });

    resolved(&text);
    assert!(text.contains('1'));
}
