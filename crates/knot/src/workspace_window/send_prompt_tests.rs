//! "Send prompt to" delivers to the chosen agent alone, queues behind a
//! session that cannot take it yet, and leaves the Changes view where it is
//! (`work-item-prompts`, #504).
//!
//! The chosen agent's panel session is still connecting, which is the case
//! `deliver_panel_prompt` queues - the same path a busy agent's prompt
//! takes - so the assertion reads the queue rather than a live adapter.

use std::sync::Arc;

use gpui_kit::TestAppContext;
use parking_lot::Mutex;

use super::WorkspaceViewMode;
use super::changes_tab::ChangesTab;
use super::render::send_prompt_menu::WorkItemRef;
use super::shortcuts_tests::window_with_agents;
use crate::panel_session::PanelSessionSlot;
use crate::workspace_window::prompt_queue::PromptOrigin;

#[gpui_kit::test]
fn the_chosen_agent_alone_queues_the_prompt_and_the_view_stays(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(2, cx);
    let (chosen, other) = (fixture.agents[0], fixture.agents[1]);
    let url = "https://github.com/acme/widget/issues/42";

    fixture.view.update(&mut fixture.window, |view, _| {
                    view.view_mode = WorkspaceViewMode::Changes;
                    view.changes_view.tab = ChangesTab::Issues;
                    let (connecting, _progress) = PanelSessionSlot::connecting();
                    view.panel_sessions
                        .insert(chosen, Arc::new(Mutex::new(connecting)));

                    assert!(view.can_receive_prompt(chosen));
                    assert!(view.send_work_item(chosen, &WorkItemRef::Issue(url.to_owned())));

                    let queue = &view.panel_prompt_queues[&chosen];
                    assert_eq!(queue.len(), 1);
                    assert_eq!(queue[0].text,
                               knot_core::l10n::t_with("changes_view.prompt.issue",
                                                       &[("url", url)]));
                    assert_eq!(queue[0].origin, PromptOrigin::User);
                    assert!(view.panel_prompt_queues
                                .get(&other)
                                .is_none_or(Vec::is_empty),
                            "no other agent receives anything");
                    assert_eq!((view.view_mode, view.changes_view.tab),
                               (WorkspaceViewMode::Changes, ChangesTab::Issues),
                               "sending leaves the view where it was");
                });
}

/// An agent that has not started has no session to take a prompt, and the
/// menu lists it disabled rather than leaving it out. The fixture's agents
/// are never activated, so neither has a session; the test only reads that,
/// since dropping a live terminal session inside an update blocks on it.
#[gpui_kit::test]
fn an_agent_without_a_session_is_listed_disabled(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    let agent = fixture.agents[0];

    fixture.view.update(&mut fixture.window, |view, _| {
                    assert!(!view.panel_sessions.contains_key(&agent)
                            && !view.sessions.contains_key(&agent),
                            "the fixture's agent has started a session, so this proves nothing");
                    let targets = view.prompt_targets();
                    assert_eq!(targets.len(), 1);
                    assert!(!targets[0].enabled);
                });
}
