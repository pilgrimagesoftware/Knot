//! The commit message window: one multi-line field, Cancel and Commit.
//!
//! A window of its own rather than the modal sheet the Swift app used.
//! `broadcast_sheet` states the convention in its own doc comment - every
//! dialog in this app that takes more than a yes/no is a window - and this is
//! the same shape as that one: a text area and two buttons.
//!
//! Unlike the broadcast sheet, this one does not close when the work is
//! handed off. A commit can fail (a rejecting hook, an empty index, a
//! configuration problem), and the message the user typed is the expensive
//! part. So the window stays until the commit succeeds, and a failure leaves
//! the text exactly where it was.
//!
//! Contract: `openspec/specs/git-panel-ui/spec.md`.

use std::sync::Arc;

use gpui_kit::base::{Disableable, StyledExt, h_flex, v_flex};
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Escape, InputEvent, Textarea, TextareaState};
use gpui_kit::{
    App, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement, Render,
    Styled, Subscription, Window, div,
};
use parking_lot::Mutex;

use crate::window_options::commit_window_options;

/// Where a commit reports back. `None` while it is still running, so the
/// window can tell "in flight" from "finished".
pub(crate) type CommitOutcome = Arc<Mutex<Option<Result<(), String>>>>;

/// What the window does with the message: hands it to the panel, which runs
/// the commit off the render path and reports back through the slot.
type CommitHandler = Box<dyn Fn(String, CommitOutcome, &mut Window, &mut App)>;

pub(crate) struct CommitWindow {
    message:       Entity<TextareaState>,
    on_commit:     CommitHandler,
    /// Set while a commit is in flight, which is what disables the button
    /// alongside an empty message.
    pending:       Option<CommitOutcome>,
    error:         Option<String>,
    _subscription: Subscription,
}

impl CommitWindow {
    /// The trimmed message, or `None` while there is nothing to commit.
    ///
    /// Trimmed of surrounding whitespace only. Internal blank lines carry the
    /// subject-then-body convention and have to survive, so this must not
    /// collapse them.
    fn trimmed(&self, cx: &App) -> Option<String> {
        let text = self.message.read(cx).value().trim().to_string();
        (!text.is_empty()).then_some(text)
    }

    fn can_commit(&self, cx: &App) -> bool {
        self.pending.is_none() && self.trimmed(cx).is_some()
    }

    fn commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_commit(cx) {
            return;
        }
        let Some(text) = self.trimmed(cx)
        else {
            return;
        };

        let outcome: CommitOutcome = Arc::new(Mutex::new(None));
        self.pending = Some(Arc::clone(&outcome));
        self.error = None;
        (self.on_commit)(text, outcome, window, cx);
        cx.notify();
    }

    /// Checks whether the commit in flight has finished, closing the window
    /// on success and keeping it - with the message intact - on failure.
    fn poll(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(slot) = self.pending.clone()
        else {
            return;
        };
        let Some(result) = slot.lock().clone()
        else {
            return;
        };

        self.pending = None;
        match result {
            Ok(()) => window.remove_window(),
            Err(reason) => {
                self.error = Some(knot_core::l10n::t_with("git_panel.commit_failed",
                                                          &[("reason", &reason)]));
                cx.notify();
            }
        }
    }
}

/// Opens the commit window, calling `on_commit` with the trimmed message.
pub(crate) fn open_commit_window(on_commit: impl Fn(String, CommitOutcome, &mut Window, &mut App)
                                 + 'static,
                                 cx: &mut App) {
    let options = commit_window_options(cx);
    let _ =
        cx.open_window(options, move |window, cx| {
              let message = cx.new(|cx| {
                                  TextareaState::new(window, cx)
                        .placeholder(knot_core::l10n::t("git_panel.commit_placeholder"))
                        .auto_grow(4, 16)
                              });
              cx.new(|cx| {
                    let subscription =
                        cx.subscribe_in(&message,
                                        window,
                                        |view: &mut CommitWindow, _, event, window, cx| {
                                            match event {
                                                // Modifier-Return commits;
                                                // plain Return inserts a
                                                // newline, since a commit
                                                // message is multi-line by
                                                // design.
                                                InputEvent::PressEnter { secondary: true, .. } => {
                                                    view.commit(window, cx);
                                                }
                                                // Commit is disabled while
                                                // the message is empty, so
                                                // it has to re-render as
                                                // the user types.
                                                InputEvent::Change => cx.notify(),
                                                _ => {}
                                            }
                                        });
                    message.update(cx, |state, cx| state.focus(window, cx));
                    CommitWindow { message,
                                   on_commit: Box::new(on_commit),
                                   pending: None,
                                   error: None,
                                   _subscription: subscription }
                })
          });
}

impl Render for CommitWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.poll(window, cx);

        let can_commit = self.can_commit(cx);
        let pending = self.pending.is_some();

        v_flex()
            .size_full()
            .gap_3()
            .px_5()
            .pt_5()
            .pb_6()
            .bg(cx.theme().background)
            // Escape cancels, discarding the message. The textarea propagates
            // the action when it has nothing of its own to dismiss, so this
            // catches it from the focused field.
            .on_action(cx.listener(|_, _: &Escape, window, _| window.remove_window()))
            .child(div().text_sm()
                        .font_semibold()
                        .child(knot_core::l10n::t("git_panel.commit_title")))
            .child(Textarea::new(&self.message).flex_1().w_full())
            .child(div().text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(knot_core::l10n::t("git_panel.commit_hint")))
            .children(self.error.clone().map(|text| {
                          div().text_xs().text_color(cx.theme().danger).child(text)
                      }))
            .child(h_flex().flex_shrink_0()
                           .justify_end()
                           .gap_2()
                           .child(Button::new("cancel-commit").label(
                    knot_core::l10n::t("git_panel.commit_cancel"),
                )
                                                              .on_click(|_, window, _| {
                                                                  window.remove_window()
                                                              }))
                           .child(Button::new("confirm-commit").label(if pending {
                                                                   knot_core::l10n::t("git_panel.committing")
                                                               }
                                                               else {
                                                                   knot_core::l10n::t("git_panel.commit")
                                                               })
                                                              .primary()
                                                              .disabled(!can_commit)
                                                              .on_click(cx.listener(
                    |view, _, window, cx| view.commit(window, cx),
                ))))
            .children(crate::app_support::root_overlays(window, cx))
    }
}
