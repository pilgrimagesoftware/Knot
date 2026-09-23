//! What the panel composer does before the rich-input change touches it.
//!
//! These are the behaviours the `Textarea` -> `Editor` migration has to
//! preserve (`openspec/changes/rich-prompt-composer`, "The composer stays a
//! composer"), written against the production constructor rather than
//! against a hand-built entity, so the swap is exercised where it happens
//! rather than where a test happens to duplicate it.
//!
//! Two of the behaviours the change names are already pinned elsewhere and
//! are not repeated here: focus on selecting a Panel-mode agent is
//! [`super::composer_focus`], and the lookup's own keys are
//! [`super::panel_lookup`].

use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::Styled;
use gpui_kit::Subscription;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::Window;
use gpui_kit::WindowOptions;
use gpui_kit::component::Root;
use gpui_kit::component::input::InputEvent;
use gpui_kit::div;

use crate::workspace_window::panel::prompt::PANEL_INPUT_ROWS_COLLAPSED;
use crate::workspace_window::panel::prompt::PANEL_INPUT_ROWS_EXPANDED;
use crate::workspace_window::panel::prompt::PanelInput;
use crate::workspace_window::panel::prompt::PanelInputState;
use crate::workspace_window::panel::prompt::new_panel_input;
use crate::workspace_window::panel::prompt::panel_input_max_rows;
use crate::workspace_window::panel::prompt::sends_on;

/// A window holding just the composer, plus the `PressEnter` events it
/// emitted.
///
/// The composer has to be drawn and focused for a keystroke to reach it at
/// all: the input's key bindings live in its own key context, so a state
/// entity with no element on screen sees nothing. That is also why this
/// probe is worth its weight - it is the only shape that can tell "Enter
/// sent" from "Enter typed a newline".
struct ComposerProbe {
    input:         Entity<PanelInputState>,
    _subscription: Subscription,
}

impl Render for ComposerProbe {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
             .child(PanelInput::new(&self.input).w_full())
    }
}

/// The `shift` flag of every `PressEnter` the composer emitted, in order.
///
/// Which chord *sends* is the window's business, not the widget's; what the
/// widget decides is which chord reports at all.
type Enters = Rc<RefCell<Vec<bool>>>;

/// A focused composer built the way the window builds it, for the given
/// send-chord setting.
fn composer(cx: &mut TestAppContext, shift_to_send: bool)
            -> (VisualTestContext, Entity<PanelInputState>, Enters) {
    let enters: Enters = Rc::new(RefCell::new(Vec::new()));
    let mut input = None;
    let window = {
        let input = &mut input;
        let enters = enters.clone();
        cx.update(|cx| {
              gpui_kit::init(cx);
              cx.open_window(WindowOptions::default(), |window, cx| {
                    let state =
                        new_panel_input(shift_to_send, panel_input_max_rows(false), window, cx);
                    *input = Some(state.clone());
                    let view = cx.new(|cx| {
                                     let subscription =
                            cx.subscribe(&state,
                                         move |_: &mut ComposerProbe,
                                               _,
                                               event: &InputEvent,
                                               _| {
                                             if let InputEvent::PressEnter { shift, .. } = event {
                                                 enters.borrow_mut().push(*shift);
                                             }
                                         });
                                     ComposerProbe { input:         state,
                                                     _subscription: subscription, }
                                 });
                    cx.new(|cx| Root::new(view, window, cx))
                })
                .expect("the composer window should open")
          })
    };
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    let input = input.expect("the composer was built");
    input.update_in(&mut cx, |state, window, cx| state.focus(window, cx));
    cx.run_until_parked();
    (cx, input, enters)
}

/// The composer's text, as the agent would receive it.
fn value(cx: &mut VisualTestContext, input: &Entity<PanelInputState>) -> String {
    cx.update(|_, cx| input.read(cx).value().to_string())
}

/// What a chord actually does to an isolated composer, for both settings.
///
/// The surprise this recorded, and the reason it is a table rather than two
/// assertions: `submit_on_enter` is **not observable from the widget**.
/// Either way round, both chords emit `PressEnter` - the emit sits outside
/// the branch in `gpui-base`'s `enter()` - and both leave a newline in the
/// buffer, because the submit branch calls `cx.propagate()` and GPUI's
/// unhandled-key path then types the return itself.
///
/// So the setting's whole effect lives in the window's subscription
/// predicate ([`sends_on`]) and in `send_panel_prompt` replacing the
/// buffer. That is exactly what the `Editor` swap must preserve, and a
/// change in any cell here is the signal that it did not.
#[gpui_kit::test]
fn each_chord_reports_and_types_the_same_either_way_round(cx: &mut TestAppContext) {
    for shift_to_send in [false, true] {
        let (mut cx, input, enters) = composer(cx, shift_to_send);

        cx.simulate_keystrokes("enter");
        cx.run_until_parked();
        assert_eq!((value(&mut cx, &input), enters.borrow().clone()),
                   ("\n".to_string(), vec![false]),
                   "Enter reports with shift=false and leaves a newline \
                    (agent_panel_shift_enter_sends={shift_to_send})");

        cx.simulate_keystrokes("shift-enter");
        cx.run_until_parked();
        assert_eq!((value(&mut cx, &input), enters.borrow().clone()),
                   ("\n\n".to_string(), vec![false, true]),
                   "Shift+Enter reports with shift=true and leaves a second newline \
                    (agent_panel_shift_enter_sends={shift_to_send})");
    }
}

/// The send chord itself, which is the predicate and nothing else.
#[test]
fn the_send_chord_follows_the_setting() {
    assert!(sends_on(false, false),
            "Enter sends when the setting is off");
    assert!(!sends_on(false, true),
            "Shift+Enter is the newline when the setting is off");
    assert!(sends_on(true, true),
            "Shift+Enter sends when the setting is on");
    assert!(!sends_on(true, false),
            "Enter is the newline when the setting is on");
}

/// Ordinary characters are unaffected by either setting - the guard that
/// says the send chord was bound, not the whole keyboard.
#[gpui_kit::test]
fn typing_reaches_the_buffer(cx: &mut TestAppContext) {
    let (mut cx, input, enters) = composer(cx, false);

    cx.simulate_keystrokes("h i");
    cx.run_until_parked();

    assert_eq!(value(&mut cx, &input), "hi");
    assert!(enters.borrow().is_empty(),
            "no keystroke here is a send chord");
}

/// The expand control re-issues the auto-grow bound, so collapsed and
/// expanded have to resolve to the two different constants - the one
/// decision both the constructor and the toggle read.
///
/// The bound is asserted at its source rather than read back off the
/// entity: `max_rows()` is `pub(super)` in `gpui-base`, so the live value
/// is not observable from here.
///
/// The ordering check is a `const` block because both sides are constants:
/// expanded has to be the larger of the two, or the control reads
/// backwards.
#[test]
fn expanding_and_collapsing_pick_the_two_row_bounds() {
    assert_eq!(panel_input_max_rows(false), PANEL_INPUT_ROWS_COLLAPSED);
    assert_eq!(panel_input_max_rows(true), PANEL_INPUT_ROWS_EXPANDED);
    const { assert!(PANEL_INPUT_ROWS_COLLAPSED < PANEL_INPUT_ROWS_EXPANDED) };
}
