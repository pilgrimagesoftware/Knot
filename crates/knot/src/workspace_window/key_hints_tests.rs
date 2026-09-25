//! The keys the sidebar shows while ⌘ is held (`agent-list-ui`): what each
//! control's hint reads, when the hints come and go, and that drawing them
//! moves nothing.

use gpui_kit::Modifiers;
use gpui_kit::TestAppContext;

use super::key_hints::SidebarKeyHints;
use super::shortcuts_tests::Fixture;
use super::shortcuts_tests::window_with;
use super::shortcuts_tests::window_with_agents;
use crate::consts::KEY_HINT_DELAY;
use crate::keymap::Chord;
use crate::keymap::Resolved;
use crate::keymap::Shortcut;

fn chord(source: &str) -> Chord {
    Chord::parse(source).expect("the test named an unparsable chord")
}

#[test]
fn the_default_hints() {
    let hints = SidebarKeyHints::from_resolved(&Resolved::defaults());
    assert_eq!(hints.dashboard, chord("cmd-alt-o").label());
    assert_eq!(hints.pull_requests, chord("cmd-alt-p").label());
    assert_eq!(hints.new_agent, chord("cmd-t").label());
    // From the family's default modifier rather than written out, which
    // `swap-select-shortcut-defaults` (#487) changes.
    let agent_modifiers = Shortcut::SelectAgent.default_modifiers()
                                               .expect("Select agent is a family");
    assert_eq!(hints.agent(0),
               Some(Chord::new(agent_modifiers, "1").label().as_str()));
    assert_eq!(hints.agent(8),
               Some(Chord::new(agent_modifiers, "9").label().as_str()));
    assert_eq!(hints.agent(9), None, "a tenth row has no shortcut");
}

#[test]
fn a_rebinding_shows_in_the_hints() {
    let resolved = Resolved::defaults().with_chord(Shortcut::ToggleDashboard, chord("ctrl-cmd-d"));
    assert_eq!(SidebarKeyHints::from_resolved(&resolved).dashboard,
               chord("ctrl-cmd-d").label());
}

fn command() -> Modifiers {
    Modifiers::command()
}

fn command_alt() -> Modifiers {
    Modifiers { platform: true,
                alt: true,
                ..Default::default() }
}

impl Fixture {
    fn hints_shown(&mut self) -> bool {
        self.view
            .read_with(&self.window, |view, _| view.key_hints.shown().is_some())
    }

    fn modifiers(&mut self, modifiers: Modifiers) {
        self.window.simulate_modifiers_change(modifiers);
        self.window.run_until_parked();
    }

    fn wait(&mut self, duration: std::time::Duration) {
        self.window.executor().advance_clock(duration);
        self.window.run_until_parked();
    }
}

#[gpui_kit::test]
fn the_hints_appear_after_the_delay_and_go_on_release(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(2, cx);
    fixture.modifiers(command());
    fixture.wait(KEY_HINT_DELAY / 2);
    assert!(!fixture.hints_shown(), "shown before the delay");
    fixture.wait(KEY_HINT_DELAY);
    assert!(fixture.hints_shown());

    fixture.modifiers(command_alt());
    assert!(fixture.hints_shown(), "adding ⌥ hid them");

    fixture.modifiers(Modifiers::none());
    assert!(!fixture.hints_shown(), "releasing ⌘ left them up");
}

#[gpui_kit::test]
fn a_quick_shortcut_does_not_flash_them(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    fixture.modifiers(command());
    fixture.window.simulate_keystrokes("cmd-c");
    fixture.wait(KEY_HINT_DELAY * 2);
    assert!(!fixture.hints_shown());
}

#[gpui_kit::test]
fn a_key_pressed_while_showing_hides_them(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    fixture.modifiers(command());
    fixture.wait(KEY_HINT_DELAY * 2);
    assert!(fixture.hints_shown());
    fixture.window.simulate_keystrokes("cmd-c");
    assert!(!fixture.hints_shown());
}

/// A delay started by an earlier hold must not show hints for a later one
/// that has not lasted long enough.
#[gpui_kit::test]
fn a_released_hold_does_not_show_them_later(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    fixture.modifiers(command());
    fixture.wait(KEY_HINT_DELAY / 2);
    fixture.modifiers(Modifiers::none());
    fixture.modifiers(command());
    fixture.wait(KEY_HINT_DELAY * 3 / 4);
    assert!(!fixture.hints_shown());
}

#[gpui_kit::test]
fn leaving_the_window_hides_them(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    fixture.modifiers(command());
    fixture.wait(KEY_HINT_DELAY * 2);
    assert!(fixture.hints_shown());
    fixture.window.deactivate_window();
    assert!(!fixture.hints_shown());
}

/// The composer usually has focus; the hold has to register through it.
#[gpui_kit::test]
fn the_hold_registers_with_the_composer_focused(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    fixture.press(crate::keymap::SelectAgent1);
    fixture.modifiers(command());
    fixture.wait(KEY_HINT_DELAY * 2);
    assert!(fixture.hints_shown());
}

#[gpui_kit::test]
fn the_hints_move_no_row(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(2, cx);
    let bounds = |fixture: &mut Fixture| {
        (fixture.window.debug_bounds("workspace-dashboard-row"),
         fixture.window.debug_bounds("workspace-agent-row-0"),
         fixture.window.debug_bounds("workspace-agent-row-1"))
    };
    let before = bounds(&mut fixture);
    assert!(before.0.is_some() && before.1.is_some() && before.2.is_some(),
            "the rows were not drawn");
    fixture.modifiers(command());
    fixture.wait(KEY_HINT_DELAY * 2);
    assert!(fixture.hints_shown());
    assert!(fixture.window.debug_bounds("key-hint").is_some(),
            "no hint was drawn, so nothing could have moved");
    assert_eq!(bounds(&mut fixture), before);
}

/// Compact rows keep their size too: there the badge sits over the avatar's
/// or icon's corner instead.
#[gpui_kit::test]
fn the_hints_move_no_compact_row(cx: &mut TestAppContext) {
    let compact_width = knot_core::consts::SIDEBAR_WIDTH_MIN;
    assert!(compact_width < knot_core::consts::SIDEBAR_COMPACT_BREAKPOINT);
    let mut fixture = window_with(2, |settings| settings.sidebar_width = compact_width, cx);
    assert!(fixture.view.read_with(&fixture.window, |view, cx| {
                            crate::workspace_window::sidebar_is_compact(view.sidebar_width(cx))
                        }),
            "the sidebar is not compact");
    let bounds = |fixture: &mut Fixture| {
        (fixture.window.debug_bounds("workspace-dashboard-row"),
         fixture.window.debug_bounds("workspace-agent-row-0"))
    };
    let before = bounds(&mut fixture);
    fixture.modifiers(command());
    fixture.wait(KEY_HINT_DELAY * 2);
    assert!(fixture.window.debug_bounds("key-hint").is_some());
    assert_eq!(bounds(&mut fixture), before);
}
