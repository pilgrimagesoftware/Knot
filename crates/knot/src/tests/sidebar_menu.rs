//! The agent list's background context menu - shape, enablement, and the
//! bulk actions it runs over a snapshot of the workspace.

use uuid::Uuid;

use crate::app_state::AgentListBackgroundEntry;
use crate::app_state::SidebarMenuFacts;
use crate::app_state::sidebar_background_menu_entries;
use crate::tests::workspace;
use crate::workspace_window::sidebar_menu_facts;
use crate::workspace_window::workspace_agent_ids;

/// The background menu's entries paired with their enabled flag - order,
/// divider placement and enablement in one value. The entry rather than its
/// label, so a copy edit in `en.yml` cannot fail these.
fn background_menu_entries(facts: SidebarMenuFacts) -> Vec<(AgentListBackgroundEntry, bool)> {
    sidebar_background_menu_entries(facts).into_iter()
                                          .map(|item| (item.entry, item.enabled))
                                          .collect()
}

/// Every item is present whatever the workspace holds - this menu disables
/// rather than omits, so its shape never changes.
#[test]
fn sidebar_background_menu_keeps_its_shape_and_offers_only_new_agent_when_empty() {
    assert_eq!(background_menu_entries(SidebarMenuFacts::default()),
               vec![(AgentListBackgroundEntry::NewAgent, true),
                    (AgentListBackgroundEntry::RestartAll, false),
                    (AgentListBackgroundEntry::CloseAll, false),
                    (AgentListBackgroundEntry::DeactivateAll, false),
                    (AgentListBackgroundEntry::Separator, false),
                    (AgentListBackgroundEntry::Broadcast, false)]);
}

#[test]
fn sidebar_background_menu_cannot_deactivate_a_workspace_of_stopped_agents() {
    assert_eq!(background_menu_entries(SidebarMenuFacts { agent_count:   3,
                                                          running_count: 0, }),
               vec![(AgentListBackgroundEntry::NewAgent, true),
                    (AgentListBackgroundEntry::RestartAll, true),
                    (AgentListBackgroundEntry::CloseAll, true),
                    (AgentListBackgroundEntry::DeactivateAll, false),
                    (AgentListBackgroundEntry::Separator, false),
                    (AgentListBackgroundEntry::Broadcast, true)]);
}

#[test]
fn sidebar_background_menu_enables_every_item_with_a_running_agent() {
    assert_eq!(background_menu_entries(SidebarMenuFacts { agent_count:   3,
                                                          running_count: 1, }),
               vec![(AgentListBackgroundEntry::NewAgent, true),
                    (AgentListBackgroundEntry::RestartAll, true),
                    (AgentListBackgroundEntry::CloseAll, true),
                    (AgentListBackgroundEntry::DeactivateAll, true),
                    (AgentListBackgroundEntry::Separator, false),
                    (AgentListBackgroundEntry::Broadcast, true)]);
}

/// A workspace of three agents, one of them owning a shell companion, in a
/// store whose current workspace is that one.
fn workspace_with_agents_and_a_companion() -> (knot_agents::AgentStore, Uuid) {
    let mut store = knot_agents::AgentStore::new();
    let space = workspace("Bulk");
    store.add_workspace(space.clone());
    store.set_current_workspace(space.id);
    let owner = store.create("~/alpha", knot_agents::CreateOptions::default());
    store.create("~/beta", knot_agents::CreateOptions::default());
    store.create("~/gamma", knot_agents::CreateOptions::default());
    store.create_shell_companion(owner).unwrap();
    (store, space.id)
}

/// The hazard this guards: `remove` takes a removed agent's companions with
/// it, so a loop reading the workspace's list as it shrinks skips agents and
/// leaves half the workspace behind. Every bulk action starts from
/// `workspace_agent_ids` for this reason.
#[test]
fn a_bulk_action_over_the_snapshot_reaches_every_agent() {
    let (mut store, workspace_id) = workspace_with_agents_and_a_companion();
    let snapshot = workspace_agent_ids(&store, workspace_id);
    assert_eq!(snapshot.len(), 4);

    for id in snapshot {
        store.remove(id);
    }

    assert!(workspace_agent_ids(&store, workspace_id).is_empty());
    assert_eq!(sidebar_menu_facts(&store, workspace_id),
               SidebarMenuFacts { agent_count:   0,
                                  running_count: 0, });
}

#[test]
fn sidebar_menu_facts_count_the_workspaces_agents_and_the_running_ones() {
    let (mut store, workspace_id) = workspace_with_agents_and_a_companion();
    assert_eq!(sidebar_menu_facts(&store, workspace_id),
               SidebarMenuFacts { agent_count:   4,
                                  running_count: 0, });

    let running = workspace_agent_ids(&store, workspace_id)[0];
    store.set_activated(running, true);
    assert_eq!(sidebar_menu_facts(&store, workspace_id),
               SidebarMenuFacts { agent_count:   4,
                                  running_count: 1, });
}

/// Every item is scoped to the workspace the sidebar is showing, so a second
/// workspace's agents are neither counted nor reached.
#[test]
fn sidebar_menu_facts_ignore_another_workspaces_agents() {
    let (mut store, workspace_id) = workspace_with_agents_and_a_companion();
    let other = workspace("Other");
    store.add_workspace(other.clone());
    store.set_current_workspace(other.id);
    store.create("~/delta", knot_agents::CreateOptions::default());

    assert_eq!(sidebar_menu_facts(&store, workspace_id).agent_count, 4);
    assert_eq!(sidebar_menu_facts(&store, other.id).agent_count, 1);
}
