//! The markdown section's review flow: Approve, Review and Submit Review
//! (`openspec/specs/artifact-panel`, "The markdown section can be approved or
//! reviewed").
//!
//! Ports the header buttons of `MarkdownPanelView`. An agent that shows a
//! plan with `display-markdown` is usually waiting on the user's verdict, and
//! these are the verdict: Approve sends the go-ahead, Review starts a reply
//! the user fills with comments and Submit Review sends it.
//!
//! Each reaches the agent by the path the user typing would take - a prompt
//! for a Panel-mode agent, keystrokes for a Terminal-mode one - so the reply
//! is recorded and queued like any other.

use std::time::Duration;

use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Window;
use uuid::Uuid;

use crate::consts;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::prompt_queue::PromptOrigin;

/// Where the user is in reviewing one markdown file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::workspace_window) enum ReviewState {
    /// Reading it. Approve and Review are offered.
    #[default]
    Viewing,
    /// A reply has been started. Submit Review is offered.
    Reviewing,
    /// A verdict was sent. Nothing is offered until the file is read again -
    /// the agent revising it is what reopens the question.
    Submitted,
}

/// The reply a Review starts, naming the file so the agent knows which of
/// its artifacts the comments below are about.
///
/// Sent to the agent rather than shown as chrome, so it is not localized:
/// the agent's instructions are in English whatever the user's locale.
pub(in crate::workspace_window) fn review_preamble(file_name: &str) -> String {
    format!("While reviewing {file_name}, user made the following comments:\n")
}

/// The markdown font size one step from `current`, or `None` at the bound.
///
/// `None` rather than a clamped value so the control can be drawn disabled
/// at the bound, which is how `MarkdownPanelView` draws it.
pub(in crate::workspace_window) fn stepped_font_size(current: i32, step: i32) -> Option<i32> {
    let next = current + step;
    (consts::MARKDOWN_FONT_SIZE_MIN..=consts::MARKDOWN_FONT_SIZE_MAX).contains(&next)
                                                                     .then_some(next)
}

impl WorkspaceWindow {
    /// `id`'s review state, `Viewing` before its file has a document.
    pub(in crate::workspace_window) fn markdown_review(&self, id: Uuid) -> ReviewState {
        self.markdown_documents
            .get(&id)
            .map(|document| document.review)
            .unwrap_or_default()
    }

    fn set_markdown_review(&mut self, id: Uuid, review: ReviewState) {
        if let Some(document) = self.markdown_documents.get_mut(&id) {
            document.review = review;
        }
    }

    /// Whether `id` is driven by a panel session rather than a terminal.
    fn is_panel_agent(&self, id: Uuid) -> bool {
        self.store
            .lock()
            .agent(id)
            .is_some_and(|agent| agent.view_mode == knot_core::ViewMode::Panel)
    }

    /// Sends the go-ahead for `id`'s file, as one submitted message.
    pub(in crate::workspace_window) fn approve_markdown(&mut self, id: Uuid,
                                                        cx: &mut Context<Self>) {
        if self.is_panel_agent(id) {
            self.deliver_panel_prompt(id,
                                      consts::ARTIFACT_APPROVE_PROMPT.to_string(),
                                      PromptOrigin::User);
        }
        else {
            self.send_terminal_command(id, consts::ARTIFACT_APPROVE_PROMPT, cx);
        }
        self.set_markdown_review(id, ReviewState::Submitted);
        cx.notify();
    }

    /// Starts a reply about `file_name`, left unsent for the user to add
    /// comments to.
    ///
    /// Into the composer for a Panel-mode agent - appended, so a draft the
    /// user had already begun is kept - and focused there, since typing the
    /// comments is the next thing to do. Typed at the prompt for a
    /// Terminal-mode agent, which is where `MarkdownPanelView` puts it.
    pub(in crate::workspace_window) fn start_markdown_review(&mut self, id: Uuid,
                                                             file_name: &str,
                                                             window: &mut Window,
                                                             cx: &mut Context<Self>) {
        let preamble = review_preamble(file_name);
        if self.is_panel_agent(id) {
            let input = self.panel_prompt_input(id, window, cx);
            cx.update_entity(&input, |state, cx| {
                  let draft = state.value().to_string();
                  let text = if draft.trim().is_empty() {
                      preamble.clone()
                  }
                  else {
                      format!("{draft}\n{preamble}")
                  };
                  state.set_value(text, window, cx);
                  state.focus(window, cx);
              });
        }
        else {
            self.send_terminal_text(id, &preamble);
        }
        self.set_markdown_review(id, ReviewState::Reviewing);
        cx.notify();
    }

    /// Sends the reply [`Self::start_markdown_review`] started.
    ///
    /// The composer's own send for a Panel-mode agent, so the comments go
    /// with any attached context exactly as pressing Send would. Escape then
    /// Return for a Terminal-mode agent: the escape dismisses an agent CLI's
    /// autocomplete, which would otherwise take the Return.
    pub(in crate::workspace_window) fn submit_markdown_review(&mut self, id: Uuid,
                                                              window: &mut Window,
                                                              cx: &mut Context<Self>) {
        if self.is_panel_agent(id) {
            self.send_panel_prompt(id, window, cx);
        }
        else {
            self.send_terminal_keys_after(id, None, cx);
        }
        self.set_markdown_review(id, ReviewState::Submitted);
        cx.notify();
    }

    /// Moves the markdown font size one step, persisting it.
    ///
    /// The same setting the panel's assistant messages are drawn at, so the
    /// conversation beside the file follows too - the two Markdown surfaces
    /// are required to render alike.
    pub(in crate::workspace_window) fn step_markdown_font_size(&mut self, step: i32,
                                                               cx: &mut Context<Self>) {
        let current = crate::settings_global::read(cx).markdown_font_size;
        let Some(next) = stepped_font_size(current, step)
        else {
            return;
        };
        let settings = crate::settings_global::write(cx, |settings| {
            settings.markdown_font_size = next;
        });
        if let Err(error) = settings.persist_preferences() {
            eprintln!("failed to persist the markdown font size: {error}");
        }
        cx.refresh_windows();
    }

    /// Types `text` at `id`'s terminal prompt without submitting it.
    fn send_terminal_text(&self, id: Uuid, text: &str) {
        let Some(session) = self.sessions.get(&id)
        else {
            return;
        };
        if let Err(error) = session.lock().send_text(text) {
            eprintln!("failed to type into agent {id}'s terminal: {error}");
        }
    }

    /// Types `text` at `id`'s terminal prompt and submits it.
    fn send_terminal_command(&self, id: Uuid, text: &str, cx: &mut Context<Self>) {
        self.send_terminal_text(id, text);
        self.send_terminal_keys_after(id, Some(consts::TERMINAL_ESCAPE_KEY_DELAY), cx);
    }

    /// Escape, then Return, each after its delay -
    /// `TerminalSessionController`'s `sendCommand` and `submitReturn`
    /// timing.
    ///
    /// `lead` is waited out before the escape: the time the text typed just
    /// before needs to reach the prompt, which a bare submit has no need of.
    fn send_terminal_keys_after(&self, id: Uuid, lead: Option<Duration>, cx: &mut Context<Self>) {
        cx.spawn(async move |view, cx| {
              if let Some(delay) = lead {
                  cx.background_executor().timer(delay).await;
              }
              view.update(cx, |view, _| view.send_terminal_text(id, "\x1b"))
                  .ok();
              cx.background_executor()
                .timer(consts::TERMINAL_RETURN_KEY_DELAY)
                .await;
              view.update(cx, |view, _| view.send_terminal_text(id, "\r"))
                  .ok();
          })
          .detach();
    }
}

#[cfg(test)]
mod tests;
