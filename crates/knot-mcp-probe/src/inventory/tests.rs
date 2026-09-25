use super::Inventory;
use crate::server::{ServerRow, Target};
use crate::state::ServerState;

fn row(name: &str, state: ServerState) -> ServerRow {
    ServerRow::new(name,
                   Target::Http { url: format!("https://{name}.example/mcp"), },
                   state)
}

/// The distinction the type exists for. Both of these have no rows to draw,
/// and the section must say something different about each.
#[test]
fn not_knowing_is_not_the_same_value_as_knowing_there_are_none() {
    let cannot_tell = Inventory::Unprobeable;
    let asked_and_found_none = Inventory::probed(vec![]);

    assert!(cannot_tell.is_unprobeable());
    assert!(!cannot_tell.found_none(),
            "an unprobeable agent has not been found to have none");

    assert!(asked_and_found_none.found_none());
    assert!(!asked_and_found_none.is_unprobeable());

    assert!(cannot_tell.rows().is_empty());
    assert!(asked_and_found_none.rows().is_empty());
}

/// Rows carry their age so the section can show it rather than implying the
/// list is live. An unprobeable agent has no age because nothing was asked.
#[test]
fn only_a_completed_probe_has_a_timestamp() {
    assert!(Inventory::Unprobeable.taken_at().is_none());
    assert!(Inventory::probed(vec![]).taken_at().is_some());
}

#[test]
fn attention_counts_only_the_unwanted_states() {
    let inventory = Inventory::probed(vec![row("a", ServerState::Connected),
                                           row("b", ServerState::NeedsAuthentication),
                                           row("c", ServerState::Failed),
                                           row("d", ServerState::Disabled),
                                           row("e", ServerState::PendingApproval),
                                           row("f", ServerState::Unknown)]);

    assert_eq!(inventory.attention_count(), 2);

    let named: Vec<_> = inventory.needing_attention()
                                 .map(|r| r.name.as_str())
                                 .collect();
    assert_eq!(named,
               ["b", "c"],
               "a disabled or pending server is a choice, not a problem");
}

#[test]
fn a_state_can_be_looked_for() {
    let inventory = Inventory::probed(vec![row("a", ServerState::Connected),
                                           row("b", ServerState::PendingApproval)]);

    assert!(inventory.any_in(ServerState::PendingApproval));
    assert!(!inventory.any_in(ServerState::Failed));
    assert!(!Inventory::Unprobeable.any_in(ServerState::Connected));
}
