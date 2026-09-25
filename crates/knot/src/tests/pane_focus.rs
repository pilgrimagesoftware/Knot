//! Where keyboard focus goes when an agent is selected
//! (`openspec/specs/acp-panel-ui`, "Selecting a Panel-mode agent focuses its
//! prompt input", and `openspec/specs/terminal-input`, "Selecting a
//! Terminal-mode agent focuses its terminal surface").
//!
//! Two subjects, and they are not the same kind of test. The dialog probe
//! below pins a property of a *dependency*: whether an open dialog sits
//! inside the window root's focus subtree, which is what decides how the
//! guard in `prepare_frame` has to be written. The rest cover
//! [`focus_target`], the pure helper that says which of the pane's input
//! targets a frame will draw.

use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::FocusHandle;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::Styled;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::Window;
use gpui_kit::WindowOptions;
use gpui_kit::component::Root;
use gpui_kit::component::WindowExt;
use gpui_kit::div;
use uuid::Uuid;

use crate::app_support;
use crate::workspace_window::pane_focus::FocusTarget;
use crate::workspace_window::pane_focus::SelectedAgentFacts;
use crate::workspace_window::pane_focus::focus_target;

/// Stands in for the workspace window's root element: one focus handle
/// tracked on the element that also hosts the overlay layers. That is
/// exactly the relationship `render/mod.rs` builds - `root_overlays` goes on
/// as a child of the element carrying `track_focus(&self.root_focus)` - and
/// it is the relationship this test is about.
struct FocusProbe {
    root: FocusHandle,
}

impl Render for FocusProbe {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
             .track_focus(&self.root)
             .children(app_support::root_overlays(window, cx))
    }
}

/// A window holding a [`FocusProbe`], and the probe itself.
fn probe_window(cx: &mut TestAppContext) -> (VisualTestContext, Entity<FocusProbe>) {
    let mut probe = None;
    let window = {
        let probe = &mut probe;
        cx.update(|cx| {
              gpui_kit::init(cx);
              cx.open_window(WindowOptions::default(), |window, cx| {
                    let view = cx.new(|cx| FocusProbe { root: cx.focus_handle(), });
                    *probe = Some(view.clone());
                    cx.new(|cx| Root::new(view, window, cx))
                })
                .expect("the probe window should open")
          })
    };
    (VisualTestContext::from_window(window.into(), cx), probe.expect("the probe was built"))
}

/// The guard `design.md` first proposed - "focus is nowhere, or inside this
/// window's own root subtree" - assumed a dialog's focus would fail the
/// containment check, on the reading that `root_overlays` adds the dialog
/// layer as a sibling of the view's own tree.
///
/// It does not. `root_overlays` is passed to `.children(..)` on the element
/// that tracks `root_focus`, so the dialog's focus handle is a descendant of
/// it in the dispatch tree, and `contains_focused` answers `true`. The guard
/// has to name the dialog explicitly instead; this test is what says so, and
/// what will fail if a gpui-component upgrade ever moves the layer out.
#[gpui_kit::test]
fn an_open_dialog_sits_inside_the_root_focus_subtree(cx: &mut TestAppContext) {
    let (mut probe_cx, probe) = probe_window(cx);

    let root = probe_cx.update(|_window, cx| probe.read(cx).root.clone());

    probe_cx.update(|window, cx| window.focus(&root, cx));
    probe_cx.run_until_parked();
    let focused_on_root = probe_cx.update(|window, cx| root.contains_focused(window, cx));
    assert!(focused_on_root,
            "the root's own focus should be inside its subtree - if this fails the probe is \
             wrong, not the dialog");

    probe_cx.update(|window, cx| {
                window.open_alert_dialog(cx, |alert, _, _| {
                          alert.title("Remove Agent")
                               .description("Standing in for any confirmation")
                               .confirm()
                      });
            });
    probe_cx.run_until_parked();

    // Without this the assertion below is vacuous: a dialog that never took
    // focus leaves it on the root, which is trivially inside the root's own
    // subtree. What makes the containment answer meaningful is that focus
    // moved somewhere else first.
    let moved = probe_cx.update(|window, cx| window.focused(cx).as_ref() != Some(&root));
    assert!(moved,
            "opening a dialog should move focus off the root - if it does not, this test proves \
             nothing about containment");

    let contained = probe_cx.update(|window, cx| root.contains_focused(window, cx));
    assert!(contained,
            "an open dialog's focus is inside the root's subtree, so a containment check cannot \
             tell a dialog apart from the window's own panes");
}

/// An activated Panel-mode agent with nothing in front of its conversation -
/// the one shape that shows a composer. Each test below spoils exactly one
/// field, so what it is about is the line it changes.
fn panel_agent() -> SelectedAgentFacts {
    SelectedAgentFacts { id:            Uuid::new_v4(),
                         is_panel_mode: true,
                         is_activated:  true,
                         has_live_grid: false, }
}

/// An activated Terminal-mode agent whose session has produced a grid - the
/// one shape that shows a terminal surface.
fn shell_agent() -> SelectedAgentFacts {
    SelectedAgentFacts { is_panel_mode: false,
                         has_live_grid: true,
                         ..panel_agent() }
}

#[test]
fn an_activated_panel_agent_shows_its_composer() {
    let agent = panel_agent();
    assert_eq!(focus_target(false, Some(&agent)),
               Some(FocusTarget::Composer(agent.id)),
               "a Panel-mode agent with its conversation on screen is the case this whole change \
                exists for");
}

#[test]
fn a_terminal_mode_agent_with_a_grid_shows_its_surface() {
    let agent = shell_agent();
    assert_eq!(focus_target(false, Some(&agent)),
               Some(FocusTarget::Terminal(agent.id)),
               "selecting a shell agent and typing with no intervening click is what this change \
                exists for");
}

/// The frame the selection changes on is not necessarily the frame the
/// surface appears on: until the session has a grid the pane draws the
/// starting placeholder, and `terminal_focus` is tracked by no element on
/// screen. Focusing it then lands outside the element tree, `prepare_frame`'s
/// own fallback moves focus to the window root on the same frame, and the
/// latch - already stored - means no later frame retries.
#[test]
fn a_terminal_mode_agent_without_a_grid_shows_nothing() {
    let agent = SelectedAgentFacts { has_live_grid: false,
                                     ..shell_agent() };
    assert_eq!(focus_target(false, Some(&agent)), None);
}

/// The grid arriving is the transition, so the frame it appears on is the one
/// that takes focus - the terminal's counterpart of
/// `closing_a_markdown_pane_shows_the_composer_again`.
#[test]
fn a_grid_arriving_is_the_transition() {
    let starting = SelectedAgentFacts { has_live_grid: false,
                                        ..shell_agent() };
    let running = SelectedAgentFacts { has_live_grid: true,
                                       ..starting };
    assert_eq!(focus_target(false, Some(&starting)), None);
    assert_eq!(focus_target(false, Some(&running)),
               Some(FocusTarget::Terminal(running.id)));
}

/// One latch over both targets, not two parallel ones: a switch between two
/// agents of different modes has to read as a transition for the one being
/// switched to, and two independent latches each see only their own half.
#[test]
fn switching_from_a_panel_agent_to_a_shell_agent_is_a_transition() {
    let panel = panel_agent();
    let shell = shell_agent();
    assert_ne!(focus_target(false, Some(&panel)),
               focus_target(false, Some(&shell)),
               "the two answers have to differ, or the latch sees no change and the terminal \
                never takes focus");
}

#[test]
fn a_deactivated_agent_shows_no_composer() {
    let agent = SelectedAgentFacts { is_activated: false,
                                     ..panel_agent() };
    assert_eq!(focus_target(false, Some(&agent)),
               None,
               "a deactivated agent draws the stopped placeholder, not a pane");
}

/// The stopped placeholder takes the content area whichever mode the agent
/// runs in, so a deactivated shell agent has no surface to focus even with a
/// grid left behind.
#[test]
fn a_deactivated_shell_agent_shows_nothing() {
    let agent = SelectedAgentFacts { is_activated: false,
                                     ..shell_agent() };
    assert_eq!(focus_target(false, Some(&agent)), None);
}

/// An open artifact is no longer a reason to withhold focus. Under
/// `artifact-panel` the panel is a sibling of the content pane, so the
/// composer is on screen beside a shown file or diagram - and whether the
/// panel is *expanded* is asked in `prepare_frame`, after the latch, not
/// here. `focus_target` answers only what the selection implies.
///
/// This reverses `an_open_markdown_file_shows_no_composer` and
/// `an_open_diagram_shows_no_composer`, which asserted the takeover these
/// facts used to encode.
#[test]
fn an_open_artifact_no_longer_withholds_the_composer() {
    let agent = panel_agent();

    assert_eq!(focus_target(false, Some(&agent)),
               Some(FocusTarget::Composer(agent.id)),
               "an artifact panel narrows the conversation rather than replacing it, so the \
                composer is there to focus");
}

/// The same for a Terminal-mode agent: an artifact does not hold its surface
/// off, because the surface is still on screen beside the panel.
#[test]
fn an_open_artifact_no_longer_withholds_the_terminal() {
    let agent = shell_agent();

    assert_eq!(focus_target(false, Some(&agent)),
               Some(FocusTarget::Terminal(agent.id)));
}

/// Both takeovers - the dashboard and the pull requests view - reach
/// `prepare_frame` through the one `is_takeover` flag, so this covers the
/// dashboard scenario the spec names and the pull requests view with it.
#[test]
fn a_takeover_view_shows_no_composer() {
    let agent = panel_agent();
    assert_eq!(focus_target(true, Some(&agent)),
               None,
               "a window showing the dashboard is not showing an agent's conversation, whatever \
                its selection is");
    assert_eq!(focus_target(true, Some(&shell_agent())),
               None,
               "nor is it showing a terminal surface");
}

#[test]
fn no_selection_shows_no_composer() {
    assert_eq!(focus_target(false, None),
               None,
               "no selected agent, or one that is not in the store, resolves the way the content \
                pane resolves it: nothing to draw");
}

/// `focus_target` carries no artifact fact at all now, so opening or closing
/// one cannot produce a transition here. That is the property the fix turns
/// on: collapsing an expanded panel must not take focus, and a guard fed into
/// this function would have latched `None` and then fired on the collapse.
#[test]
fn an_artifact_opening_or_closing_is_not_a_transition() {
    let agent = panel_agent();

    let before = focus_target(false, Some(&agent));
    let after = focus_target(false, Some(&agent));

    assert_eq!(before, after,
               "the answer depends on the selection alone, so nothing an artifact does can \
                change it");
    assert_eq!(before, Some(FocusTarget::Composer(agent.id)));
}
