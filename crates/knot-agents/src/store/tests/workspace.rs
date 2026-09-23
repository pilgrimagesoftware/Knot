use super::super::*;

fn workspace(name: &str) -> knot_core::Workspace {
    knot_core::Workspace { id:        Uuid::new_v4(),
                           name:      name.to_string(),
                           color_hex: "#000000".to_string(),
                           agent_ids: Vec::new(), }
}

#[test]
fn workspace_reordering_moves_before_target() {
    let first = workspace("first");
    let second = workspace("second");
    let third = workspace("third");
    let mut store = AgentStore::new();
    store.add_workspace(first.clone());
    store.add_workspace(second.clone());
    store.add_workspace(third.clone());
    assert!(store.move_workspace_before(third.id, first.id));
    assert_eq!(store.workspaces()
                    .iter()
                    .map(|workspace| workspace.id)
                    .collect::<Vec<_>>(),
               vec![third.id, first.id, second.id]);
}

#[test]
fn removing_workspace_keeps_one_and_moves_agent() {
    let first = workspace("first");
    let second = workspace("second");
    let mut store = AgentStore::new();
    store.add_workspace(first.clone());
    store.add_workspace(second.clone());
    store.set_current_workspace(first.id);
    assert!(store.remove_workspace(first.id));
    assert_eq!(store.current_workspace_id(), Some(second.id));
    assert!(!store.remove_workspace(second.id));
}

#[test]
fn reorder_and_move_to_workspace_update_membership() {
    let mut store = AgentStore::new();
    let first = store.create("/tmp/first", CreateOptions::default());
    let second = store.create("/tmp/second", CreateOptions::default());
    let source = store.workspaces()[0].id;
    store.reorder(source, 0, 1);
    assert_eq!(store.workspaces()[0].agent_ids, vec![second, first]);
    let target = workspace("target");
    let target_id = target.id;
    store.add_workspace(target);
    store.move_to_workspace(first, target_id);
    assert!(!store.workspaces()[0].agent_ids.contains(&first));
    assert_eq!(store.workspaces()
                    .iter()
                    .find(|workspace| workspace.id == target_id)
                    .unwrap()
                    .agent_ids,
               vec![first]);
}

/// Deleting a workspace takes its arrangement with it, in the teardown rather
/// than only on the next load: a window reopened before the next launch must
/// not read a dead workspace's bounds.
#[test]
fn removing_a_workspace_drops_its_ui_state() {
    let mut store = AgentStore::new();
    let keep = workspace("Keep");
    let drop = workspace("Drop");
    let (keep_id, drop_id) = (keep.id, drop.id);
    store.add_workspace(keep);
    store.add_workspace(drop);
    store.set_workspace_window_bounds(keep_id,
                                      knot_core::SavedWindowBounds { x:      1.0,
                                                                     y:      2.0,
                                                                     width:  3.0,
                                                                     height: 4.0, });
    store.set_workspace_window_bounds(drop_id,
                                      knot_core::SavedWindowBounds { x:      9.0,
                                                                     y:      9.0,
                                                                     width:  9.0,
                                                                     height: 9.0, });

    assert!(store.remove_workspace(drop_id));

    assert!(store.saved_workspace_ui().contains_key(&keep_id));
    assert!(!store.saved_workspace_ui().contains_key(&drop_id),
            "the removed workspace left its arrangement behind");
    assert_eq!(store.workspace_ui(drop_id),
               knot_core::WorkspaceUiState::default(),
               "and reading it back gives the default, not the dead frame");
}

/// A repeated bounds write reports no change, which is what keeps a pointer
/// drag from persisting once per frame.
#[test]
fn setting_the_same_bounds_twice_reports_no_change() {
    let mut store = AgentStore::new();
    let ws = workspace("One");
    let id = ws.id;
    store.add_workspace(ws);
    let bounds = knot_core::SavedWindowBounds { x:      10.0,
                                                y:      20.0,
                                                width:  800.0,
                                                height: 600.0, };

    assert!(store.set_workspace_window_bounds(id, bounds),
            "first write changed it");
    assert!(!store.set_workspace_window_bounds(id, bounds),
            "second write did not");
}
