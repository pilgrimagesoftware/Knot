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
//!
//! Since the swap to `EditorState`, this module also carries
//! `panel-rich-input`'s "The composer stays a composer" - the guard that
//! the widget underneath is an editor and the thing on screen is not.

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
use tempfile::TempDir;

use crate::composer_scan::Construct;
use crate::composer_style::ComposerStyling;
use crate::composer_style::Palette;
use crate::workspace_window::panel::composer::new_panel_input;
use crate::workspace_window::panel::composer::panel_input_max_rows;
use crate::workspace_window::panel::composer::sends_now;
use crate::workspace_window::panel::composer::sends_on;
use crate::workspace_window::panel::prompt::PANEL_INPUT_ROWS_COLLAPSED;
use crate::workspace_window::panel::prompt::PANEL_INPUT_ROWS_EXPANDED;
use crate::workspace_window::panel::prompt::PanelInput;
use crate::workspace_window::panel::prompt::PanelInputState;

/// The sources the guard below reads, relative to the crate root.
const COMPOSER_SOURCE: &str = "src/workspace_window/panel/composer.rs";
const RENDER_SOURCE: &str = "src/workspace_window/render/mod.rs";
const REPAINT_SOURCE: &str = "src/workspace_window/repaint.rs";

/// One of those sources, with every run of whitespace removed.
///
/// A guard that quotes a line of Rust is otherwise only as good as
/// `rustfmt`'s current line breaks: the form it forbids can come back
/// wrapped differently and the `contains` then passes vacuously. Haystack
/// and needle both go through this, so the assertions are about the tokens
/// rather than the layout.
fn squeezed(relative: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| {
                                                 panic!("reading {}: {error}", path.display())
                                             });

    text.split_whitespace().collect()
}

/// A needle squeezed the way [`squeezed`] squeezes its haystack.
fn needle(form: &str) -> String {
    form.split_whitespace().collect()
}

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

/// The same predicate against the live surface, which is what the
/// subscription actually consults (#429).
///
/// A composer is built once per agent and then handed back from the cache
/// for the rest of that agent's life, so anything the subscription captures
/// is frozen at whatever the setting said the first time that panel was
/// shown. The hint below the prompt box and the Send button tooltip read the
/// surface on every frame and moved as soon as the setting changed; the keys
/// did not, which is the whole of the reported defect.
///
/// Rooted at a temporary directory rather than `Settings::default()`:
/// nothing here persists, but a default carries no store paths, and one
/// careless `persist` in a later edit would write over the developer's own
/// workspaces and agents.
#[gpui_kit::test]
fn the_send_chord_follows_a_setting_changed_after_the_composer_was_built(cx: &mut TestAppContext) {
    let dir = TempDir::new().expect("a temporary settings root");

    cx.update(|cx| {
          gpui_kit::init(cx);
          let mut settings = knot_core::Settings::with_store_root(dir.path());
          settings.agent_panel_shift_enter_sends = false;
          crate::settings_global::install(settings, cx);

          // A composer built while the setting is off - the entity whose
          // subscription used to fix the chord for the rest of its life.
          cx.open_window(WindowOptions::default(), |window, cx| {
                let state = new_panel_input(false, panel_input_max_rows(false), window, cx);
                cx.new(|cx| {
                      let subscription =
                          cx.subscribe(&state, |_: &mut ComposerProbe, _, _: &InputEvent, _| {});
                      ComposerProbe { input:         state,
                                      _subscription: subscription, }
                  })
            })
            .expect("the composer window should open");

          assert!(sends_now(false, cx), "Enter sends while the setting is off");
          assert!(!sends_now(true, cx),
                  "Shift+Enter is the newline while the setting is off");

          crate::settings_global::write(cx, |settings| {
              settings.agent_panel_shift_enter_sends = true
          });

          assert!(sends_now(true, cx),
                  "Shift+Enter has to send once the setting is on, for the composer that was \
                   already open as much as for one built afterwards");
          assert!(!sends_now(false, cx),
                  "and Enter has to stop sending - otherwise both chords send and the hint is \
                   telling the truth about neither");
      });
}

/// The wiring the test above cannot see: that the subscription reads the
/// surface instead of a captured flag, and that an entity already built is
/// brought back in line rather than left holding the old `submit_on_enter`.
///
/// A source guard, for the same reason as `mcp_panel::probe::tests`': the
/// decision happens inside a closure reached only by a real keystroke on a
/// real panel session, so its absence has no other output channel. The
/// defect this pins passed every test in this module.
#[test]
fn the_subscription_reads_the_setting_rather_than_capturing_it() {
    let composer_src = squeezed(COMPOSER_SOURCE);

    assert!(composer_src.contains(&needle("if sends_now(*shift, cx)")),
            "the PressEnter guard must consult the live surface; a captured flag makes the send \
             chord whatever it was when the composer was built");
    assert!(!composer_src.contains(&needle("sends_on(shift_to_send, *shift)")),
            "the captured-flag form is #429 exactly, and it compiles");
}

/// The other half of the wiring, and the reason it hangs off the poll rather
/// than off a render.
///
/// `settings_global::write` notifies nobody, so a preference change reaches
/// this window only when something asks - and a render is not something that
/// is guaranteed to happen. `repaint_poll_tick` notifies only when one of
/// its flags is true, so on a quiet workspace it can be never, which is the
/// "could be never" the poll's own `pull_request_states` comment already
/// describes. Reconciling from a render leaves a window in which `sends_now`
/// is live and the widget is not, and in that window the chord the hint
/// calls "newline" neither sends nor inserts one.
#[test]
fn the_send_chord_is_reconciled_from_the_repaint_poll() {
    let repaint = squeezed(REPAINT_SOURCE);
    let render = squeezed(RENDER_SOURCE);
    let composer_src = squeezed(COMPOSER_SOURCE);
    let call = needle("self.reconcile_panel_send_chord(cx)");

    assert!(repaint.contains(&call),
            "repaint_poll_tick must reconcile, or a composer built before the change keeps the \
             old submit_on_enter until something unrelated happens to redraw the window");
    assert!(repaint.contains(&needle("|| send_chord_changed")),
            "and its answer must be in the notify chain, or the frame that would show the \
             reconciled composer is never asked for");
    assert!(!render.contains(&call) && !composer_src.contains(&call),
            "reconciling from a render makes the fix wait for a repaint nothing schedules");
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

/// A composer is an editor that is not laid out as one, and the whole of
/// "no line numbers, no gutter, no indent guides, no fold controls" rests
/// on that: every one of them is a field of `LayoutMode::CodeEditor`, which
/// [`new_panel_input`] leaves by calling `auto_grow` last.
///
/// `is_code_editor()` cannot be asked - it answers from the mode *marker*,
/// so it is `true` for any `EditorState` however it is laid out. What the
/// layout decides is observable through behaviour instead, which is what
/// the tests below do.
#[gpui_kit::test]
fn the_composer_is_an_editor_that_is_not_laid_out_as_one(cx: &mut TestAppContext) {
    let (mut cx, input, _) = composer(cx, false);

    assert!(cx.update(|_, cx| input.read(cx).is_code_editor()),
            "the state is an EditorState - that is what carries decorations, and the reason for \
             the swap");
    assert!(cx.update(|_, cx| input.read(cx).is_multi_line()),
            "and it is multi-line, which is what makes the row sizing apply at all");
}

/// `panel-rich-input`: "A bracket is not auto-closed".
///
/// Auto-closing is keyed to the layout (`LayoutMode::is_auto_close`), so
/// leaving the code-editor layout is what turns it off - no flag is set.
#[gpui_kit::test]
fn typing_an_opening_bracket_leaves_it_alone(cx: &mut TestAppContext) {
    let (mut cx, input, _) = composer(cx, false);

    cx.simulate_keystrokes("(");
    cx.run_until_parked();

    assert_eq!(value(&mut cx, &input),
               "(",
               "a composer that closes brackets is a code editor wearing a composer's layout");
}

/// `panel-rich-input`: the composer must not "re-indent on newline".
///
/// This one is not free. `enter()` upstream decides to indent from
/// `self.is_code_editor()` - the mode marker, not the layout - so before
/// the fork's second commit a newline here inherited the previous line's
/// indent, and `"    indented"` became `"    indented\n    "`. The fork
/// keys it to the layout, like the auto-close beside it; this is what says
/// so, and what fails if a bump loses that commit.
#[gpui_kit::test]
fn a_newline_does_not_inherit_the_previous_lines_indent(cx: &mut TestAppContext) {
    let (mut cx, input, _) = composer(cx, false);

    input.update_in(&mut cx, |state, window, cx| {
             state.set_value("    indented", window, cx);
             let end = state.value().len();
             state.set_selected_range(end..end, cx);
         });
    cx.run_until_parked();

    cx.simulate_keystrokes("shift-enter");
    cx.run_until_parked();

    assert_eq!(value(&mut cx, &input),
               "    indented\n",
               "the newline is bare - a prompt is prose, and what the user typed is what the \
                agent receives");
}

/// A prompt is written left to right and wrapped, not scrolled sideways.
/// Soft wrap is `gpui-base`'s default for a multi-line input and the
/// composer never turns it off; this is the guard on that default.
#[gpui_kit::test]
fn a_long_line_is_not_turned_into_a_horizontal_scroll(cx: &mut TestAppContext) {
    let (mut cx, input, _) = composer(cx, false);

    let long = "wrap ".repeat(200);
    input.update_in(&mut cx, |state, window, cx| {
             state.set_value(long.clone(), window, cx);
         });
    cx.run_until_parked();

    assert_eq!(value(&mut cx, &input),
               long,
               "wrapping is presentation: the buffer holds one line however it is drawn");
}

/// The styling for a composer holding `text`, and the composer itself.
///
/// Built the way the window builds it: the text is in the buffer *before*
/// the styling exists, which is the shape a restored draft arrives in.
fn styled(cx: &mut TestAppContext, text: &str)
          -> (VisualTestContext, Entity<PanelInputState>, ComposerStyling) {
    let (mut cx, input, _) = composer(cx, false);
    let text = text.to_string();
    input.update_in(&mut cx, |state, window, cx| {
             state.set_value(text, window, cx)
         });
    cx.run_until_parked();
    let styling = cx.update(|_, cx| {
                        let palette = Palette::of(cx);
                        ComposerStyling::new(&input, &[], palette, cx)
                    });
    (cx, input, styling)
}

/// Which constructs the styling currently paints.
fn constructs(styling: &ComposerStyling) -> Vec<Construct> {
    let mut found: Vec<Construct> = styling.spans().iter().map(|span| span.construct).collect();
    found.sort_unstable();
    found.dedup();
    found
}

/// `panel-rich-input`: "A restored draft is styled" - without the user
/// editing it, which is what makes this a test about arrival rather than
/// about change.
#[gpui_kit::test]
fn a_restored_draft_is_styled_on_arrival(cx: &mut TestAppContext) {
    let (_cx, _input, styling) = styled(cx, "# Plan\n\nSend `make lint` and **fix** it.");

    assert_eq!(constructs(&styling),
               vec![Construct::Strong, Construct::InlineCode, Construct::Heading],
               "every construct in the restored text should be styled with no edit to trigger it");
}

/// `panel-rich-input`: "Styling survives every way the composer changes".
///
/// Typing, a multi-line paste and a cut are all one path - the buffer
/// changed - so what this checks is that the path is actually taken and
/// lands on the same answer a fresh scan would.
#[gpui_kit::test]
fn styling_follows_the_buffer_however_it_changes(cx: &mut TestAppContext) {
    let (mut cx, input, mut styling) = styled(cx, "plain prose");
    assert!(constructs(&styling).is_empty(),
            "the fixture starts unstyled");

    // Typed.
    cx.simulate_keystrokes("space * a *");
    cx.run_until_parked();
    let changed = cx.update(|_, cx| styling.on_change(&input, &[], cx));
    assert!(changed,
            "typing changed the buffer, so it has to have restyled");
    assert_eq!(constructs(&styling), vec![Construct::Emphasis]);

    // Pasted, as a whole document arriving at once.
    input.update_in(&mut cx, |state, window, cx| {
             state.set_value("## Title\n\n- one\n\n```\ncode\n```".to_string(),
                             window,
                             cx);
         });
    cx.run_until_parked();
    cx.update(|_, cx| styling.on_change(&input, &[], cx));
    assert_eq!(constructs(&styling),
               vec![Construct::CodeFence,
                    Construct::Heading,
                    Construct::ListMarker],
               "a pasted document is styled as though it had been typed");

    // Cut back to nothing.
    input.update_in(&mut cx, |state, window, cx| {
             state.set_value(String::new(), window, cx);
         });
    cx.run_until_parked();
    cx.update(|_, cx| styling.on_change(&input, &[], cx));
    assert!(constructs(&styling).is_empty(),
            "an emptied buffer keeps no spans from what used to be in it");
}

/// A buffer that did not change must not cost a rescan - this is the
/// guard on the composer re-rendering per keystroke.
#[gpui_kit::test]
fn an_unchanged_buffer_is_not_restyled(cx: &mut TestAppContext) {
    let (mut cx, input, mut styling) = styled(cx, "**unchanged**");

    let changed = cx.update(|_, cx| styling.on_change(&input, &[], cx));

    assert!(!changed,
            "nothing changed, so nothing should have been repainted");
}

/// `panel-rich-input`: "Styling never changes the text".
///
/// The strongest form available without a live agent: the buffer is
/// compared byte for byte across a full styling pass, markers included.
/// That the *agent* receives it unchanged follows from `send_panel_prompt`
/// reading this same value, and is walked through by hand in a debug
/// build.
#[gpui_kit::test]
fn styling_leaves_the_buffer_byte_identical(cx: &mut TestAppContext) {
    let original = "# Heading\n\n**ship it** with `code`, a [link](a.md) and /review @lib.rs";
    let (mut cx, input, mut styling) = styled(cx, original);

    assert!(!constructs(&styling).is_empty(),
            "the fixture has to be styled, or this is vacuous");

    cx.update(|_, cx| styling.on_change(&input, &[], cx));
    let after = value(&mut cx, &input);

    assert_eq!(after, original,
               "styling is presentation: not one marker character may be added, removed or moved");
}

/// An appearance switch repaints without touching the spans, which is what
/// lets it hang off the frame GPUI already draws.
#[gpui_kit::test]
fn an_appearance_switch_repaints_without_rescanning(cx: &mut TestAppContext) {
    let (mut cx, _input, mut styling) = styled(cx, "**ship it**");
    let before = styling.spans().to_vec();

    let same = cx.update(|_, cx| {
                     let palette = Palette::of(cx);
                     styling.on_palette(palette, cx)
                 });
    assert!(!same, "the same palette is not a repaint");

    let flipped = cx.update(|_, cx| {
                        let mut palette = Palette::of(cx);
                        palette.is_dark = !palette.is_dark;
                        palette.foreground = gpui_kit::hsla(0.5, 0.5, 0.5, 1.);
                        styling.on_palette(palette, cx)
                    });

    assert!(flipped, "a different appearance has to repaint");
    assert_eq!(styling.spans(),
               before,
               "the spans do not depend on the theme, so a repaint must not have moved them");
}
