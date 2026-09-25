//! A library prompt in the `/` lookup: listed as a prompt, and written in
//! place of the slash token - or dropped when the buffer moved on while it
//! resolved. Run against the composer the window builds, as
//! `tests::panel_lookup` does, since the caret and the surrounding text are
//! what the spec is about.

use gpui_kit::AppContext;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::WindowOptions;
use gpui_kit::component::Root;
use gpui_kit::{Context, Entity, IntoElement, Render, Window, div};

use super::apply_expansion;
use crate::panel_commands::{LookupEntry, Matcher, active_token};
use crate::workspace_window::panel::prompt::{
    PanelInputState, new_panel_input, panel_input_max_rows,
};

struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

fn composer(cx: &mut TestAppContext, value: &str) -> (VisualTestContext, Entity<PanelInputState>) {
    let window = cx.update(|cx| {
                       gpui_kit::init(cx);
                       cx.open_window(WindowOptions::default(), |window, cx| {
                             let view = cx.new(|_| Blank);
                             cx.new(|cx| Root::new(view, window, cx))
                         })
                         .expect("failed to open the test window")
                   });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    let input =
        cx.update(|window, cx| new_panel_input(false, panel_input_max_rows(false), window, cx));
    let value = value.to_string();
    let caret = value.len();
    input.update_in(&mut cx, |state, window, cx| {
             state.set_value(value, window, cx);
             state.set_selected_range(caret..caret, cx);
         });
    (cx, input)
}

#[test]
fn a_library_prompt_is_listed_by_name_and_marked_as_a_prompt() {
    let gate = knot_core::Prompt::new("Run the gate", "make,\nthen commit").unwrap();
    let entries = [LookupEntry::library_prompt(&gate)];

    let matches = Matcher::Substring.matching(&entries, "gate");

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].entry.token, "Run the gate");
    assert_eq!(matches[0].entry.description, "make, then commit");
    assert_eq!(matches[0].entry.prompt, Some(gate.id));
    assert!(LookupEntry::new("send", "x").prompt.is_none());
}

/// The spec's example: a `/gat` token on the second line is replaced by the
/// prompt's text, and the first line survives.
#[gpui_kit::test]
fn the_expansion_replaces_the_token_in_place(cx: &mut TestAppContext) {
    let (mut cx, input) = composer(cx, "note:\n/gat");
    let (snapshot, range) = cx.update(|_, cx| {
                                  let state = input.read(cx);
                                  let token = active_token(&state.value(), state.cursor())
                                      .expect("an active token");
                                  (state.value().to_string(), token.range)
                              });

    cx.update(|window, cx| {
          apply_expansion(&input, &snapshot, range, "make, then commit", window, cx)
      });

    assert_eq!(cx.update(|_, cx| input.read(cx).value().to_string()),
               "note:\nmake, then commit");
}

/// An edit made while the expansion resolved wins; the expansion is dropped.
#[gpui_kit::test]
fn a_stale_expansion_is_dropped(cx: &mut TestAppContext) {
    let (mut cx, input) = composer(cx, "/gat");
    let (snapshot, range) = cx.update(|_, cx| {
                                  let state = input.read(cx);
                                  let token = active_token(&state.value(), state.cursor())
                                      .expect("an active token");
                                  (state.value().to_string(), token.range)
                              });
    input.update_in(&mut cx, |state, window, cx| {
             state.set_value("/gate typed on", window, cx)
         });

    cx.update(|window, cx| apply_expansion(&input, &snapshot, range, "expanded", window, cx));

    assert_eq!(cx.update(|_, cx| input.read(cx).value().to_string()),
               "/gate typed on");
}
