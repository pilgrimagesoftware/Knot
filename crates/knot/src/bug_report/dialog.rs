//! The bug-report dialog: the action that opens it, the entity that holds
//! what was typed, and the dialog built over it.
//!
//! Hosted by `window.open_dialog`, as the workspace name dialog is, so it
//! inherits the overlay, focus trap and Escape-to-cancel. The builder runs
//! inside `Root`'s render and reads only [`BugReport`] and its inputs - never
//! the window's own view, which is mid-update then.
//!
//! Collecting diagnostics and submitting both block on subprocesses, so both
//! run on the background executor and land back through `update_in`, which
//! notifies the entity and so reaches a frame without any poll.

use std::sync::Arc;

use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::ClickEvent;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::Subscription;
use gpui_kit::WeakEntity;
use gpui_kit::Window;
use gpui_kit::base::Disableable;
use gpui_kit::base::StyledExt;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::WindowExt;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::dialog::Cancel;
use gpui_kit::component::dialog::Confirm;
use gpui_kit::component::dialog::Dialog;
use gpui_kit::component::dialog::DialogFooter;
use gpui_kit::component::input::Input;
use gpui_kit::component::input::InputEvent;
use gpui_kit::component::input::InputState;
use gpui_kit::component::input::Textarea;
use gpui_kit::component::input::TextareaState;
use gpui_kit::component::notification::Notification;
use gpui_kit::div;
use gpui_kit::px;
use knot_forge::ForgeAvailability;

use super::diagnostics::Diagnostics;
use super::form::{Phase, can_report};
use super::submit::{self, Outcome, Report};
use crate::app_bootstrap::ReportBug;
use crate::consts::BUG_REPORT_DIALOG_WIDTH;

type Collect = dyn Fn() -> Diagnostics + Send + Sync;
type Submit = dyn Fn(&Report, &ForgeAvailability) -> Outcome + Send + Sync;

/// The two blocking operations the dialog calls, behind a seam so the window
/// tests neither shell out to `gh` nor open a browser.
#[derive(Clone)]
pub(crate) struct ReportServices {
    pub(crate) collect: Arc<Collect>,
    pub(crate) submit:  Arc<Submit>,
}

impl ReportServices {
    /// The real host, the real `gh`, the real browser.
    fn live() -> Self {
        Self { collect: Arc::new(Diagnostics::collect),
               submit:  Arc::new(|report, forge| {
                   submit::submit(report,
                                  forge,
                                  &knot_forge::GhRunner::new(),
                                  crate::open_in::open_url)
               }), }
    }
}

/// The open dialog, if there is one: the single-instance handle.
///
/// Weak, because the dialog host owns the entity through the builder it
/// keeps. Closing the dialog drops the builder, the entity with it, and so
/// this stops upgrading - no close observer has to clear it.
struct OpenReport {
    services: ReportServices,
    window:   Option<gpui_kit::AnyWindowHandle>,
    report:   Option<WeakEntity<BugReport>>,
}

impl gpui_kit::Global for OpenReport {}

/// Registers the `ReportBug` handler over the live services.
pub(crate) fn register_report_bug_action(cx: &mut App) {
    register_report_bug_action_with(ReportServices::live(), cx);
}

/// [`register_report_bug_action`], over supplied services.
pub(crate) fn register_report_bug_action_with(services: ReportServices, cx: &mut App) {
    cx.set_global(OpenReport { services,
                               window: None,
                               report: None });
    cx.on_action(|_: &ReportBug, cx| open_report_dialog(cx));
}

/// Raises the open dialog's window, or opens the dialog on the active one.
///
/// Deferred, like `quit_guard::request_quit`: the menu dispatch runs inside
/// the active window's update, and a re-entrant `window.update` is reported
/// as a missing window. The single-instance check runs inside the deferral
/// too, so two quick menu picks cannot both pass it before either opens.
fn open_report_dialog(cx: &mut App) {
    cx.defer(|cx| {
          let open = cx.global::<OpenReport>();
          if let (Some(window), Some(report)) = (open.window, open.report.as_ref())
             && report.upgrade().is_some()
             && window.update(cx, |_, window, _| window.activate_window())
                      .is_ok()
          {
              return;
          }
          let Some(window) = cx.active_window().or_else(|| cx.windows().first().copied())
          else {
              // Nothing to attach to. The menu has no other way to answer,
              // and a report is not worth an app-level window of its own.
              eprintln!("bug report: there is no window to open the dialog on");
              return;
          };
          let services = cx.global::<OpenReport>().services.clone();
          let result = window.update(cx, |_, window, cx| open_on(services, window, cx));
          match result {
              Ok(report) => {
                  let open = cx.global_mut::<OpenReport>();
                  open.window = Some(window);
                  open.report = Some(report.downgrade());
              }
              Err(error) => eprintln!("bug report: the window went away before it opened: {error}"),
          }
      });
}

/// The open dialog's report, if one is open.
#[cfg(test)]
pub(crate) fn open_report(cx: &App) -> Option<Entity<BugReport>> {
    cx.try_global::<OpenReport>()?.report.as_ref()?.upgrade()
}

fn open_on(services: ReportServices, window: &mut Window, cx: &mut App) -> Entity<BugReport> {
    let report = cx.new(|cx| BugReport::new(services, window, cx));
    window.open_dialog(cx, {
              let report = report.clone();
              move |dialog, _, app| build_dialog(dialog, &report, app)
          });
    // After `open_dialog`, which focuses its own handle: the dialog
    // autofocuses its first field.
    report.update(cx, |report, cx| {
              report.subject
                    .update(cx, |input, cx| input.focus(window, cx));
              report.collect(window, cx);
          });
    report
}

/// What the dialog holds while it is open. Created with it and dropped with
/// it, so a report never outlives its dialog and the next one starts empty.
pub(crate) struct BugReport {
    pub(crate) subject:     Entity<InputState>,
    pub(crate) description: Entity<TextareaState>,
    pub(crate) diagnostics: Entity<TextareaState>,
    phase:                  Phase,
    services:               ReportServices,
    _subscriptions:         Vec<Subscription>,
}

impl BugReport {
    fn new(services: ReportServices, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subject = cx.new(|cx| {
                            InputState::new(window, cx)
                                .placeholder(knot_core::l10n::t("bug_report.subject_placeholder"))
                        });
        let description = cx.new(|cx| {
                                TextareaState::new(window, cx)
                    .placeholder(knot_core::l10n::t("bug_report.description_placeholder"))
                    .auto_grow(5, 12)
                            });
        let diagnostics =
            cx.new(|cx| {
                  let mut state = TextareaState::new(window, cx).auto_grow(4, 6);
                  state.set_value(knot_core::l10n::t("bug_report.diagnostics_collecting"),
                                  window,
                                  cx);
                  state
              });
        // Report's enabled state follows the fields, so a keystroke has to
        // re-render the dialog rather than wait for an unrelated repaint.
        let subscriptions = vec![cx.subscribe(&subject, |_, _, event: &InputEvent, cx| {
                                       if matches!(event, InputEvent::Change) {
                                           cx.notify();
                                       }
                                   }),
                                 cx.subscribe(&description, |_, _, event: &InputEvent, cx| {
                                       if matches!(event, InputEvent::Change) {
                                           cx.notify();
                                       }
                                   })];
        Self { subject,
               description,
               diagnostics,
               phase: Phase::Editing,
               services,
               _subscriptions: subscriptions }
    }

    /// Whether Report can be chosen right now.
    pub(crate) fn can_report(&self, cx: &App) -> bool {
        can_report(&self.subject.read(cx).value(),
                   &self.description.read(cx).value(),
                   &self.phase)
    }

    /// Fills the diagnostics pane off the main thread.
    fn collect(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let collect = Arc::clone(&self.services.collect);
        cx.spawn_in(window, async move |this, cx| {
              let diagnostics = cx.background_executor()
                                  .spawn(async move { collect() })
                                  .await;
              let _ = this.update_in(cx, |report, window, cx| {
                              report.show_diagnostics(&diagnostics, window, cx);
                          });
          })
          .detach();
    }

    fn show_diagnostics(&mut self, diagnostics: &Diagnostics, window: &mut Window,
                        cx: &mut Context<Self>) {
        let text = diagnostics.text();
        self.diagnostics
            .update(cx, |state, cx| state.set_value(text, window, cx));
        cx.notify();
    }

    /// Files the report, or hands it to the browser.
    ///
    /// Diagnostics are collected again rather than read back off the pane:
    /// the forge may have been signed in since the dialog opened, and the
    /// probe is what decides the path. The pane is refreshed with what was
    /// sent, so what the user sees is what the issue carries.
    pub(crate) fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.can_report(cx) {
            return;
        }
        self.phase = Phase::Submitting;
        cx.notify();
        let subject = self.subject.read(cx).value().to_string();
        let description = self.description.read(cx).value().to_string();
        let services = self.services.clone();
        cx.spawn_in(window, async move |this, cx| {
              let (diagnostics, outcome) =
                  cx.background_executor()
                    .spawn(async move {
                        let diagnostics = (services.collect)();
                        let report = Report { subject,
                                              description,
                                              diagnostics: diagnostics.text() };
                        let outcome = (services.submit)(&report, &diagnostics.forge);
                        (diagnostics, outcome)
                    })
                    .await;
              let _ = this.update_in(cx, |report, window, cx| {
                              report.show_diagnostics(&diagnostics, window, cx);
                              report.finish(outcome, window, cx);
                          });
          })
          .detach();
    }

    fn finish(&mut self, outcome: Outcome, window: &mut Window, cx: &mut Context<Self>) {
        self.phase = match outcome {
            Outcome::Filed(url) => {
                window.close_dialog(cx);
                window.push_notification(Notification::success(knot_core::l10n::t_with("bug_report.filed",
                                                                                        &[("url", &url)])),
                                         cx);
                Phase::Editing
            }
            Outcome::FileFailed(error) => Phase::Failed(error),
            Outcome::BrowserReady => Phase::BrowserReady,
            Outcome::BrowserFailed => Phase::BrowserFailed,
        };
        cx.notify();
    }
}

/// Rebuilt every frame by the dialog layer, which is what lets Report's
/// enabled state and the status line follow the entity.
fn build_dialog(dialog: Dialog, report: &Entity<BugReport>, app: &mut App) -> Dialog {
    let enabled = report.read(app).can_report(app);
    let submitting = report.read(app).phase == Phase::Submitting;

    dialog.w(px(BUG_REPORT_DIALOG_WIDTH))
          .close_button(false)
          // A stray click on the backdrop must not discard a typed report.
          // Escape still cancels, deliberately.
          .overlay_closable(false)
          .title(knot_core::l10n::t("bug_report.title"))
          .content({
              let report = report.clone();
              move |content, _, app| content.child(dialog_content(&report, app))
          })
          .footer(dialog_footer(enabled, submitting))
          .on_ok({
              let report = report.clone();
              move |_, window, app| {
                  report.update(app, |report, cx| report.submit(window, cx));
                  // The submission closes the dialog itself once the issue
                  // is filed; every other outcome keeps it open.
                  false
              }
          })
          .on_cancel(|_, _, _| true)
}

fn dialog_content(report: &Entity<BugReport>, app: &App) -> impl IntoElement {
    let report = report.read(app);
    let theme = app.theme();
    let label = |key: &str| {
        div().text_sm()
             .font_semibold()
             .child(knot_core::l10n::t(key))
    };

    v_flex().gap_2()
            .child(label("bug_report.subject_label"))
            .child(Input::new(&report.subject))
            .child(label("bug_report.description_label"))
            .child(Textarea::new(&report.description).w_full())
            .child(label("bug_report.diagnostics_label"))
            .child(div().text_xs()
                        .text_color(theme.muted_foreground)
                        .child(knot_core::l10n::t("bug_report.diagnostics_hint")))
            // Read-only rather than disabled: it keeps its normal look and
            // can still be focused, selected and copied.
            .child(Textarea::new(&report.diagnostics).readonly(true)
                                                     .w_full()
                                                     .text_xs())
            .children(report.phase.message().map(|message| {
                                                div().text_sm()
                                                     .text_color(if report.phase.is_error() {
                                                                     theme.danger
                                                                 }
                                                                 else {
                                                                     theme.muted_foreground
                                                                 })
                                                     .child(message)
                                            }))
}

/// Cancel and Report dispatch the host's own actions, so the buttons, Return
/// and Escape all reach the same `on_ok` / `on_cancel`.
fn dialog_footer(enabled: bool, submitting: bool) -> impl IntoElement {
    DialogFooter::new()
        .child(Button::new("cancel-bug-report").label(knot_core::l10n::t("bug_report.cancel"))
                                               .on_click(|_: &ClickEvent, window, app| {
                                                   window.dispatch_action(Box::new(Cancel), app);
                                               }))
        .child(Button::new("submit-bug-report").label(knot_core::l10n::t("bug_report.report"))
                                               .primary()
                                               .loading(submitting)
                                               .disabled(!enabled)
                                               .on_click(|_: &ClickEvent, window, app| {
                                                   window.dispatch_action(Box::new(Confirm { secondary: false }), app);
                                               }))
}
