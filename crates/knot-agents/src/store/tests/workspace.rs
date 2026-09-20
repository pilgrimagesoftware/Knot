use super::super::*;

fn workspace(name: &str) -> knot_core::Workspace {
    knot_core::Workspace {
        id: Uuid::new_v4(),
        name: name.to_string(),
        color_hex: "#000000".to_string(),
        agent_ids: Vec::new(),
        active_agent_ids: Vec::new(),
        layout_mode: "single".to_string(),
        focused_pane_index: 0,
        split_ratio: 0.5,
        split_ratio_secondary: None,
        show_dashboard: None,
        is_detached: None,
        window_bounds: None,
    }
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
    assert_eq!(
        store
            .workspaces()
            .iter()
            .map(|workspace| workspace.id)
            .collect::<Vec<_>>(),
        vec![third.id, first.id, second.id]
    );
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
    assert_eq!(
        store
            .workspaces()
            .iter()
            .find(|workspace| workspace.id == target_id)
            .unwrap()
            .agent_ids,
        vec![first]
    );
}
