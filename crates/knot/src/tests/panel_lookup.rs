//! Inserting a slash-lookup entry rewrites the token and nothing else
//! (`openspec/specs/panel-slash-commands`, "Inserting an entry completes
//! text").
//!
//! The design flagged surgical replacement as this change's one real risk:
//! the textarea has no replace-a-range call, so the lookup selects the
//! token's span and replaces the selection instead. These assertions run
//! that pair against a real `TextareaState`, which is the only way to know
//! the caret lands where the spec says and the surrounding buffer survives.

use gpui_kit::AppContext;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::WindowOptions;
use gpui_kit::component::Root;
use gpui_kit::component::input::TextareaState;
use gpui_kit::{Context, Entity, IntoElement, Render, Window, div};

use crate::panel_commands::active_token;
use crate::workspace_window::panel::lookup::replace_lookup_token;

/// A root view with no content: these assertions read a textarea's buffer,
/// not a rendered frame.
struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// A prompt textarea holding `value` with the caret at byte offset `caret`.
fn prompt(cx: &mut TestAppContext, value: &str, caret: usize)
          -> (VisualTestContext, Entity<TextareaState>) {
    let window = cx.update(|cx| {
                       gpui_kit::init(cx);
                       cx.open_window(WindowOptions::default(), |window, cx| {
                             let view = cx.new(|_| Blank);
                             cx.new(|cx| Root::new(view, window, cx))
                         })
                         .expect("failed to open the test window")
                   });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    let input = cx.update(|window, cx| cx.new(|cx| TextareaState::new(window, cx)));
    let value = value.to_string();
    input.update_in(&mut cx, |state, window, cx| {
             state.set_value(value, window, cx);
             state.set_selected_range(caret..caret, cx);
         });
    (cx, input)
}

/// The spec's own example: `/sen` completed to `send` leaves the rest of
/// the line alone.
#[gpui_kit::test]
fn inserting_replaces_the_token_in_place(cx: &mut TestAppContext) {
    let (mut cx, input) = prompt(cx, "/sen the plan to Ada", 4);

    let token = cx.update(|_, cx| {
                      let state = input.read(cx);
                      active_token(&state.value(), state.cursor()).expect("an active token")
                  });
    cx.update(|window, cx| replace_lookup_token(&input, token.range.clone(), "send", window, cx));

    let (value, cursor) = cx.update(|_, cx| {
                                let state = input.read(cx);
                                (state.value().to_string(), state.cursor())
                            });
    assert_eq!(value, "/send the plan to Ada");
    assert_eq!(cursor, 5,
               "the caret should land just after the inserted token");
}

/// A token filling the whole buffer completes to just the token.
#[gpui_kit::test]
fn inserting_over_a_lone_token_leaves_only_the_token(cx: &mut TestAppContext) {
    let (mut cx, input) = prompt(cx, "/wor", 4);

    let token = cx.update(|_, cx| {
                      let state = input.read(cx);
                      active_token(&state.value(), state.cursor()).expect("an active token")
                  });
    cx.update(|window, cx| {
          replace_lookup_token(&input, token.range.clone(), "worktree", window, cx)
      });

    let value = cx.update(|_, cx| input.read(cx).value().to_string());
    assert_eq!(value, "/worktree");
}

/// Text on earlier lines is outside the token's span and must survive it.
#[gpui_kit::test]
fn inserting_leaves_the_rest_of_the_buffer_untouched(cx: &mut TestAppContext) {
    let (mut cx, input) = prompt(cx, "first line\n/che", 15);

    let token = cx.update(|_, cx| {
                      let state = input.read(cx);
                      active_token(&state.value(), state.cursor()).expect("an active token")
                  });
    cx.update(|window, cx| replace_lookup_token(&input, token.range.clone(), "check", window, cx));

    let value = cx.update(|_, cx| input.read(cx).value().to_string());
    assert_eq!(value, "first line\n/check");
}
