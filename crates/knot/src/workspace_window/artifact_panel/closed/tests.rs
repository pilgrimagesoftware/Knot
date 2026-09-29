//! Closing and reopening the artifact panel: what a close remembers, and
//! what a reopen may and may not put back.
//!
//! The pure half pins [`ClosedArtifacts::reopenable`]; the window half drives
//! the close and reopen paths against a real workspace window's store, since
//! the store is where the panel's presence is decided.

use std::path::PathBuf;

use gpui_kit::TestAppContext;

use super::CloseTarget;
use super::ClosedArtifacts;
use super::ClosedDiagram;
use crate::keymap::SelectAgent1;
use crate::keymap::ToggleArtifacts;
use crate::workspace_window::artifact_panel::state::ArtifactSnapshot;
use crate::workspace_window::shortcuts_tests::available;
use crate::workspace_window::shortcuts_tests::window_with_agents;

fn file(name: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/{name}.md"))
}

fn diagram(source: &str) -> ClosedDiagram {
    ClosedDiagram { source: source.to_string(),
                    title:  Some("Plan".to_string()), }
}

#[test]
fn nothing_closed_has_nothing_to_reopen() {
    assert!(ClosedArtifacts::default().reopenable(&ArtifactSnapshot::default())
                                      .is_empty());
}

#[test]
fn a_closed_section_with_an_empty_slot_is_reopenable() {
    let closed = ClosedArtifacts { markdown: Some(file("plan")),
                                   mermaid:  Some(diagram("graph TD; A-->B;")), };

    let restore = closed.reopenable(&ArtifactSnapshot::default());

    assert_eq!(restore, closed);
}

/// The agent showed something newer after the user closed the old one; the
/// reopen control must not swap the old one back over it.
#[test]
fn a_section_the_agent_has_refilled_is_not_reopened() {
    let closed = ClosedArtifacts { markdown: Some(file("old")),
                                   mermaid:  Some(diagram("graph TD; A-->B;")), };
    let open = ArtifactSnapshot { markdown:  Some(file("new")),
                                  maximized: false,
                                  mermaid:   None, };

    let restore = closed.reopenable(&open);

    assert_eq!(restore.markdown, None);
    assert_eq!(restore.mermaid, closed.mermaid);
}

fn show_both(fixture: &mut crate::workspace_window::shortcuts_tests::Fixture) -> uuid::Uuid {
    let id = fixture.agents[0];
    fixture.view.update(&mut fixture.window, |view, cx| {
                    {
                        let mut store = view.store.lock();
                        store.set_markdown_panel(id, file("plan"), true)
                             .expect("the agent exists");
                        store.set_mermaid_panel(id,
                                                "graph TD; A-->B;".to_string(),
                                                Some("Plan".to_string()))
                             .expect("the agent exists");
                    }
                    // A frame, so the window's handlers reflect the artifact
                    // - what the repaint poll's notify would do in the app.
                    cx.notify();
                });
    id
}

#[gpui_kit::test]
fn close_all_then_reopen_restores_both_sections(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let id = show_both(&mut fixture);

    fixture.view.update(&mut fixture.window, |view, _| {
                    assert!(view.reopenable_artifacts(id).is_empty(),
                            "nothing is closed while both are open");
                    view.close_artifact_sections(id, CloseTarget::Both);
                    assert!(!view.artifact_panel_open(id), "close-all closes the panel");
                    assert!(!view.reopenable_artifacts(id).is_empty(),
                            "a closed panel offers to come back");

                    view.reopen_artifacts(id);

                    let snapshot = view.artifact_snapshot(id);
                    assert_eq!(snapshot.markdown, Some(file("plan")));
                    assert!(!snapshot.maximized,
                            "reopening asks for nothing about the panel's size");
                    assert_eq!(snapshot.mermaid.as_deref(), Some("graph TD; A-->B;"));
                    let title = view.store
                                    .lock()
                                    .agent(id)
                                    .and_then(|agent| agent.mermaid_title.clone());
                    assert_eq!(title.as_deref(),
                               Some("Plan"),
                               "the diagram comes back with its title");
                    assert!(view.reopenable_artifacts(id).is_empty(),
                            "once reopened there is nothing left to reopen");
                });
}

#[gpui_kit::test]
fn closing_one_section_remembers_only_that_one(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let id = show_both(&mut fixture);

    fixture.view.update(&mut fixture.window, |view, _| {
                    view.close_artifact_sections(id, CloseTarget::Mermaid);

                    let restore = view.reopenable_artifacts(id);
                    assert_eq!(restore.markdown, None, "the markdown section never closed");
                    assert!(restore.mermaid.is_some());
                    assert!(view.artifact_panel_open(id),
                            "the markdown section still holds the panel open");
                });
}

#[gpui_kit::test]
fn forgetting_the_agent_forgets_what_it_closed(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let id = show_both(&mut fixture);

    fixture.view.update(&mut fixture.window, |view, _| {
                    view.close_artifact_sections(id, CloseTarget::Both);
                    view.forget_artifact_panel(id);

                    assert!(view.reopenable_artifacts(id).is_empty());
                });
}

/// View > Artifacts and its shortcut have nothing to do for an agent that has
/// never had an artifact, so the item is drawn disabled.
#[gpui_kit::test]
fn toggle_artifacts_is_unavailable_without_an_artifact(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    fixture.press(SelectAgent1);

    assert!(!available(&mut fixture, &ToggleArtifacts));
}

/// The shortcut hides a shown panel and brings the same panel back, through
/// the window's own handler - the path the menu item dispatches to.
#[gpui_kit::test]
fn toggle_artifacts_hides_and_reshows_the_panel(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    fixture.press(SelectAgent1);
    let id = show_both(&mut fixture);
    fixture.window.run_until_parked();
    let open = |fixture: &mut crate::workspace_window::shortcuts_tests::Fixture| {
        fixture.view
               .read_with(&fixture.window, |view, _| view.artifact_panel_open(id))
    };
    assert!(available(&mut fixture, &ToggleArtifacts));

    fixture.press(ToggleArtifacts);
    assert!(!open(&mut fixture), "the shortcut hides a shown panel");
    assert!(available(&mut fixture, &ToggleArtifacts),
            "a hidden panel can still be brought back");

    fixture.press(ToggleArtifacts);
    assert!(open(&mut fixture), "and brings it back");
    let snapshot = fixture.view
                          .read_with(&fixture.window, |view, _| view.artifact_snapshot(id));
    assert_eq!(snapshot.markdown, Some(file("plan")));
    assert_eq!(snapshot.mermaid.as_deref(), Some("graph TD; A-->B;"));
}
