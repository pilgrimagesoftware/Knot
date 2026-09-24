//! Inserting a slash-lookup entry rewrites the token and nothing else
//! (`openspec/specs/panel-slash-commands`, "Inserting an entry completes
//! text").
//!
//! The design flagged surgical replacement as this change's one real risk:
//! the input has no replace-a-range call, so the lookup selects the token's
//! span and replaces the selection instead. These assertions run that pair
//! against the composer the window actually builds, which is the only way
//! to know the caret lands where the spec says and the surrounding buffer
//! survives.

use gpui_kit::AppContext;
use gpui_kit::TestAppContext;
use gpui_kit::VisualTestContext;
use gpui_kit::WindowOptions;
use gpui_kit::component::Root;
use gpui_kit::{Context, Entity, IntoElement, Render, Window, div};

use crate::panel_commands::Trigger;
use crate::panel_commands::active_token;
use crate::workspace_window::panel::composer::new_panel_input;
use crate::workspace_window::panel::composer::panel_input_max_rows;
use crate::workspace_window::panel::lookup::replace_lookup_token;
use crate::workspace_window::panel::prompt::PanelInputState;

/// A root view with no content: these assertions read the composer's
/// buffer, not a rendered frame.
struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// A composer holding `value` with the caret at byte offset `caret`.
///
/// Built through [`new_panel_input`] rather than from the state type
/// directly, so surgical replacement is exercised against the composer the
/// window actually creates - including its layout mode, which decides what
/// an edit has to step over.
fn prompt(cx: &mut TestAppContext, value: &str, caret: usize)
          -> (VisualTestContext, Entity<PanelInputState>) {
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
    cx.update(|window, cx| {
          replace_lookup_token(&input,
                               token.range.clone(),
                               Trigger::Slash,
                               "send",
                               window,
                               cx)
      });

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
          replace_lookup_token(&input,
                               token.range.clone(),
                               Trigger::Slash,
                               "worktree",
                               window,
                               cx)
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
    cx.update(|window, cx| {
          replace_lookup_token(&input,
                               token.range.clone(),
                               Trigger::Slash,
                               "check",
                               window,
                               cx)
      });

    let value = cx.update(|_, cx| input.read(cx).value().to_string());
    assert_eq!(value, "first line\n/check");
}

/// `panel-file-mentions`' own example, and task 6.8's check: the typed
/// token is replaced and the words around it survive.
#[gpui_kit::test]
fn inserting_a_mention_replaces_only_the_typed_token(cx: &mut TestAppContext) {
    let (mut cx, input) = prompt(cx, "look at @knotg", 14);

    let token = cx.update(|_, cx| {
                      let state = input.read(cx);
                      active_token(&state.value(), state.cursor()).expect("an active token")
                  });
    assert_eq!(token.trigger, Trigger::Mention);

    cx.update(|window, cx| {
          replace_lookup_token(&input,
                               token.range.clone(),
                               Trigger::Mention,
                               "crates/knot-git/src/lib.rs",
                               window,
                               cx)
      });

    let value = cx.update(|_, cx| input.read(cx).value().to_string());
    assert_eq!(value, "look at @crates/knot-git/src/lib.rs");
}

/// A mention is text and nothing else. Inserting one attaches no context
/// and reads no file - an image's path in the prompt is a path, not an
/// attachment, which is what keeps `@` from being a second paperclip.
#[gpui_kit::test]
fn inserting_an_image_path_attaches_nothing(cx: &mut TestAppContext) {
    let (mut cx, input) = prompt(cx, "@shot", 5);

    let token = cx.update(|_, cx| {
                      let state = input.read(cx);
                      active_token(&state.value(), state.cursor()).expect("an active token")
                  });
    cx.update(|window, cx| {
          replace_lookup_token(&input,
                               token.range.clone(),
                               Trigger::Mention,
                               "images/screenshot.png",
                               window,
                               cx)
      });

    let value = cx.update(|_, cx| input.read(cx).value().to_string());
    assert_eq!(value, "@images/screenshot.png",
               "the buffer gained a path; nothing read the file and nothing attached it");
}

/// A path with a space is inserted escaped, so it stays one token.
#[gpui_kit::test]
fn inserting_a_path_with_a_space_keeps_it_one_token(cx: &mut TestAppContext) {
    let (mut cx, input) = prompt(cx, "@my", 3);

    let token = cx.update(|_, cx| {
                      let state = input.read(cx);
                      active_token(&state.value(), state.cursor()).expect("an active token")
                  });
    cx.update(|window, cx| {
          replace_lookup_token(&input,
                               token.range.clone(),
                               Trigger::Mention,
                               "my notes/today.md",
                               window,
                               cx)
      });

    let value = cx.update(|_, cx| input.read(cx).value().to_string());
    assert_eq!(value, r"@my\ notes/today.md");

    let reparsed = cx.update(|_, cx| {
                         let state = input.read(cx);
                         active_token(&state.value(), state.value().len())
                     });
    assert_eq!(reparsed.map(|token| token.range),
               Some(0..value.len()),
               "the inserted mention has to read back as one token, or the composer styles half \
                of it");
}

/// A slash command is not escaped: it has no whitespace to protect, and
/// backslashes in the buffer would only make it harder to read.
#[gpui_kit::test]
fn inserting_a_command_does_not_escape_it(cx: &mut TestAppContext) {
    let (mut cx, input) = prompt(cx, "/rev", 4);

    let token = cx.update(|_, cx| {
                      let state = input.read(cx);
                      active_token(&state.value(), state.cursor()).expect("an active token")
                  });
    cx.update(|window, cx| {
          replace_lookup_token(&input,
                               token.range.clone(),
                               Trigger::Slash,
                               "review",
                               window,
                               cx)
      });

    assert_eq!(cx.update(|_, cx| input.read(cx).value().to_string()),
               "/review");
}
