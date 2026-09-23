//! What a workspace window names itself, per `agent-list-ui`'s "The
//! sidebar's title bar names the workspace".
//!
//! The resolver is tested rather than the element it feeds: it carries the
//! whole decision - which workspace, what text, and what a missing one
//! produces - so a rename that stops reaching the window fails here without
//! a window having to be stood up to look at.

use uuid::Uuid;

use crate::tests::workspace;
use crate::workspace_window::workspace_title;

#[test]
fn a_workspace_resolves_to_its_name() {
    let mut store = knot_agents::AgentStore::new();
    let payments = workspace("Payments");
    let id = payments.id;
    store.add_workspace(payments);

    assert_eq!(workspace_title(&store, id), Some("Payments".to_string()));
}

/// The point of reading the store per frame rather than snapshotting the
/// name at open: the same store, renamed, answers with the new name.
#[test]
fn a_renamed_workspace_resolves_to_its_new_name() {
    let mut store = knot_agents::AgentStore::new();
    let payments = workspace("Payments");
    let id = payments.id;
    store.add_workspace(payments);
    assert!(store.rename_workspace(id, "Billing"));

    assert_eq!(workspace_title(&store, id), Some("Billing".to_string()));
}

/// The deleted-workspace window's case. It gets no fallback string because
/// it never draws one - it returns early with `workspace.missing`.
#[test]
fn an_unknown_id_resolves_to_nothing() {
    let mut store = knot_agents::AgentStore::new();
    store.add_workspace(workspace("Payments"));

    assert_eq!(workspace_title(&store, Uuid::new_v4()), None);
}

/// A workspace name is user data and the title bar declares one line, so the
/// resolver flattens rather than trusting `whitespace_nowrap` to - which it
/// would not, per `single_line`.
#[test]
fn a_name_carrying_a_newline_resolves_to_one_line() {
    let mut store = knot_agents::AgentStore::new();
    let wrapped = workspace("Payments\nand refunds");
    let id = wrapped.id;
    store.add_workspace(wrapped);

    assert_eq!(workspace_title(&store, id),
               Some("Payments and refunds".to_string()));
}
