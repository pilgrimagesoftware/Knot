//! Replying to one of the agent's responses (#559): the response action
//! bar's Reply control quotes the response into the composer as a Markdown
//! block quote, and leaves the caret on the line below it for the reply.
//!
//! The quoting is pure and tested on its own; [`insert_reply_quote`] is the
//! one place that writes it into a composer, so the window and the tests go
//! through the same edit.

use gpui_kit::Context;
use gpui_kit::Window;
use uuid::Uuid;

use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::panel::prompt::PanelInputState;

/// `response` as a Markdown block quote, ending on an empty line.
///
/// Every line is quoted, blank ones as a bare `>` so the quote stays one
/// block rather than splitting into several. It ends with a blank line
/// because Markdown's lazy continuation would otherwise fold a reply typed
/// straight after the last `> ` line into the quote itself.
pub(crate) fn quote_for_reply(response: &str) -> String {
    let mut quote = String::new();
    // Only the blank lines around the response go, not the indent of its
    // first line: a response opening with an indented code block keeps it.
    for line in response.trim_end().trim_start_matches(['\n', '\r']).lines() {
        let line = line.trim_end();
        if line.is_empty() {
            quote.push('>');
        }
        else {
            quote.push_str("> ");
            quote.push_str(line);
        }
        quote.push('\n');
    }
    quote.push('\n');
    quote
}

/// What to write after `draft` so the quote starts on a paragraph of its
/// own: nothing for an empty draft, otherwise enough newlines to leave one
/// blank line between the two.
fn separator_after(draft: &str) -> &'static str {
    if draft.trim().is_empty() || draft.ends_with("\n\n") {
        ""
    }
    else if draft.ends_with('\n') {
        "\n"
    }
    else {
        "\n\n"
    }
}

/// Appends `response`, quoted, to the composer's text and leaves the caret
/// at the end, on the line below the quote.
///
/// Appended rather than replacing, so a draft the user had begun is kept;
/// a draft that is only whitespace is replaced. Written with
/// `set_selected_range` + `replace` rather than `set_value`, which puts a
/// multi-line input's caret back at the start of the text.
pub(crate) fn insert_reply_quote(state: &mut PanelInputState, response: &str,
                                 window: &mut Window, cx: &mut Context<PanelInputState>) {
    let draft = state.value().to_string();
    let end = draft.len();
    let range = if draft.trim().is_empty() {
        0..end
    }
    else {
        end..end
    };
    let text = format!("{}{}", separator_after(&draft), quote_for_reply(response));
    state.set_selected_range(range, cx);
    state.replace(text, window, cx);
    state.focus(window, cx);
}

impl WorkspaceWindow {
    /// Quotes `response` into agent `id`'s composer and focuses it, ready
    /// for the reply.
    pub(in crate::workspace_window) fn reply_to_response(&mut self, id: Uuid, response: &str,
                                                         window: &mut Window,
                                                         cx: &mut Context<Self>) {
        let input = self.panel_prompt_input(id, window, cx);
        input.update(cx, |state, cx| {
                 insert_reply_quote(state, response, window, cx)
             });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_line_is_quoted_and_the_quote_ends_on_an_empty_line() {
        assert_eq!(quote_for_reply("first\nsecond"), "> first\n> second\n\n");
    }

    /// A bare `>` keeps a multi-paragraph response one quote; an unquoted
    /// blank line would end it there.
    #[test]
    fn a_blank_line_inside_the_response_stays_inside_the_quote() {
        assert_eq!(quote_for_reply("one\n\ntwo"), "> one\n>\n> two\n\n");
    }

    #[test]
    fn surrounding_blank_lines_are_not_quoted_but_indentation_is_kept() {
        assert_eq!(quote_for_reply("\n\n    code\ntext  \n\n"),
                   ">     code\n> text\n\n");
    }

    #[test]
    fn a_draft_is_kept_a_paragraph_away_from_the_quote() {
        assert_eq!(separator_after(""), "");
        assert_eq!(separator_after("  \n"), "");
        assert_eq!(separator_after("draft"), "\n\n");
        assert_eq!(separator_after("draft\n"), "\n");
        assert_eq!(separator_after("draft\n\n"), "");
    }

    #[test]
    fn the_reply_tooltip_resolves() {
        let key = "panel.reply_to_response";
        assert_ne!(knot_core::l10n::t(key), key);
    }
}
