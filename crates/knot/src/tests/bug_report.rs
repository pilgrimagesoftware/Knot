//! Help > Report a Bug opens one dialog on the active window, and the dialog
//! keeps or files the report as `bug-reporting`'s spec says.
//!
//! The menu path is driven through `TestAppContext::dispatch_action`, which
//! runs the handler inside the window's update as macOS does - the reason
//! the open is deferred. The services are stubbed, so nothing here runs `gh`
//! or opens a browser.

use std::sync::Arc;

use gpui_kit::component::Root;
use gpui_kit::component::WindowExt;
use gpui_kit::{
    AnyWindowHandle, AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled,
    TestAppContext, VisualTestContext, Window, WindowOptions, div,
};
use knot_forge::ForgeAvailability;
use parking_lot::Mutex;

use crate::app_bootstrap::ReportBug;
use crate::bug_report::{
    BugReport, Diagnostics, Outcome, Report, ReportServices, open_report,
    register_report_bug_action_with,
};

/// A root view with no content but the overlay layers, which every Knot root
/// view renders - without them a dialog is pushed onto the `Root` and never
/// drawn.
struct Blank;

impl Render for Blank {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full()
             .children(crate::app_support::root_overlays(window, cx))
    }
}

/// What the stub submission answers, and every report it was handed.
#[derive(Clone)]
struct Stub {
    outcome:   Arc<Mutex<Outcome>>,
    submitted: Arc<Mutex<Vec<Report>>>,
}

impl Stub {
    fn answering(outcome: Outcome) -> Self {
        Self { outcome:   Arc::new(Mutex::new(outcome)),
               submitted: Arc::new(Mutex::new(Vec::new())), }
    }

    fn services(&self) -> ReportServices {
        let outcome = Arc::clone(&self.outcome);
        let submitted = Arc::clone(&self.submitted);
        ReportServices { collect: Arc::new(|| {
                             Diagnostics { app:   "Knot 1.0.0 (2026-09-25, abc)".to_owned(),
                                           os:    "macOS 26.0".to_owned(),
                                           arch:  "aarch64".to_owned(),
                                           forge: ForgeAvailability::Ready, }
                         }),
                         submit:  Arc::new(move |report, _| {
                             submitted.lock().push(report.clone());
                             outcome.lock().clone()
                         }), }
    }
}

fn app_with_one_window(cx: &mut TestAppContext, stub: &Stub) -> AnyWindowHandle {
    cx.update(|cx| {
          gpui_kit::init(cx);
          register_report_bug_action_with(stub.services(), cx);
          cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| Blank);
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("failed to open the test window")
            .into()
      })
}

fn open_dialog(cx: &mut TestAppContext, stub: &Stub) -> (VisualTestContext, Entity<BugReport>) {
    let handle = app_with_one_window(cx, stub);
    cx.dispatch_action(handle, ReportBug);
    cx.run_until_parked();
    let report = cx.update(|cx| open_report(cx))
                   .expect("Report a Bug opened no dialog");
    (VisualTestContext::from_window(handle, cx), report)
}

fn dialog_is_open(cx: &mut VisualTestContext) -> bool {
    cx.update(|window, cx| window.has_active_dialog(cx))
}

fn fill(cx: &mut VisualTestContext, report: &Entity<BugReport>, subject: &str, description: &str) {
    report.update_in(cx, |report, window, cx| {
              report.subject.update(cx, |input, cx| {
                                input.set_value(subject.to_owned(), window, cx)
                            });
              report.description.update(cx, |input, cx| {
                                    input.set_value(description.to_owned(), window, cx)
                                });
          });
    cx.run_until_parked();
}

fn submit(cx: &mut VisualTestContext, report: &Entity<BugReport>) {
    report.update_in(cx, |report, window, cx| report.submit(window, cx));
    cx.run_until_parked();
}

#[gpui_kit::test]
fn the_menu_opens_the_dialog_on_the_active_window(cx: &mut TestAppContext) {
    let (mut cx, _report) = open_dialog(cx, &Stub::answering(Outcome::BrowserReady));

    assert!(dialog_is_open(&mut cx),
            "Report a Bug put nothing on the window");
    assert!(cx.debug_bounds("dialog-layer").is_some(),
            "the dialog layer never reached the screen");
}

#[gpui_kit::test]
fn choosing_the_item_again_keeps_the_one_dialog(cx: &mut TestAppContext) {
    let stub = Stub::answering(Outcome::BrowserReady);
    let handle = app_with_one_window(cx, &stub);
    cx.dispatch_action(handle, ReportBug);
    cx.run_until_parked();
    let first = cx.update(|cx| open_report(cx)).expect("no dialog opened");

    cx.dispatch_action(handle, ReportBug);
    cx.run_until_parked();

    let second = cx.update(|cx| open_report(cx))
                   .expect("the dialog went away");
    assert_eq!(first.entity_id(),
               second.entity_id(),
               "a second dialog opened");
}

#[gpui_kit::test]
fn the_diagnostics_are_filled_in_on_open(cx: &mut TestAppContext) {
    let (mut cx, report) = open_dialog(cx, &Stub::answering(Outcome::BrowserReady));

    let text = cx.update(|_, cx| report.read(cx).diagnostics.read(cx).value().to_string());
    assert!(text.contains("Knot 1.0.0") && text.contains("aarch64"),
            "{text}");
}

#[gpui_kit::test]
fn escape_cancels_and_files_nothing(cx: &mut TestAppContext) {
    let stub = Stub::answering(Outcome::BrowserReady);
    let (mut cx, report) = open_dialog(cx, &stub);
    fill(&mut cx, &report, "Crash", "It crashed.");
    drop(report);

    cx.simulate_keystrokes("escape");
    cx.run_until_parked();

    assert!(!dialog_is_open(&mut cx), "escape left the dialog open");
    assert!(stub.submitted.lock().is_empty(), "escape filed a report");
    assert!(cx.update(|_, cx| open_report(cx)).is_none(),
            "the closed dialog's report outlived it");
}

#[gpui_kit::test]
fn report_needs_both_fields(cx: &mut TestAppContext) {
    let (mut cx, report) = open_dialog(cx, &Stub::answering(Outcome::BrowserReady));

    fill(&mut cx, &report, "Crash", "   ");
    assert!(!cx.update(|_, cx| report.read(cx).can_report(cx)));

    fill(&mut cx, &report, "Crash", "It crashed.");
    assert!(cx.update(|_, cx| report.read(cx).can_report(cx)));
}

#[gpui_kit::test]
fn a_filed_issue_closes_the_dialog(cx: &mut TestAppContext) {
    let url = "https://github.com/pilgrimagesoftware/Knot/issues/7".to_owned();
    let stub = Stub::answering(Outcome::Filed(url));
    let (mut cx, report) = open_dialog(cx, &stub);
    fill(&mut cx, &report, "Crash", "It crashed.");

    submit(&mut cx, &report);

    assert!(!dialog_is_open(&mut cx),
            "a filed report left the dialog open");
    let submitted = stub.submitted.lock();
    assert_eq!(submitted.len(), 1);
    assert_eq!(submitted[0].subject, "Crash");
    assert_eq!(submitted[0].description, "It crashed.");
    assert!(submitted[0].diagnostics.contains("Knot 1.0.0"));
}

#[gpui_kit::test]
fn a_failure_keeps_the_report_and_allows_a_retry(cx: &mut TestAppContext) {
    let stub = Stub::answering(Outcome::FileFailed("HTTP 403".to_owned()));
    let (mut cx, report) = open_dialog(cx, &stub);
    fill(&mut cx, &report, "Crash", "It crashed.");

    submit(&mut cx, &report);

    assert!(dialog_is_open(&mut cx),
            "a failed submission closed the dialog");
    let (subject, description, retry) = cx.update(|_, cx| {
                                              let report = report.read(cx);
                                              (report.subject.read(cx).value().to_string(),
                                               report.description.read(cx).value().to_string(),
                                               report.can_report(cx))
                                          });
    assert_eq!((subject.as_str(), description.as_str()),
               ("Crash", "It crashed."));
    assert!(retry, "Report stayed disabled after a failure");

    submit(&mut cx, &report);
    assert_eq!(stub.submitted.lock().len(),
               2,
               "the retry was not submitted");
}

fn assert_hand_off_keeps_the_dialog(cx: &mut TestAppContext, outcome: Outcome) {
    let stub = Stub::answering(outcome.clone());
    let (mut cx, report) = open_dialog(cx, &stub);
    fill(&mut cx, &report, "Crash", "It crashed.");

    submit(&mut cx, &report);

    assert!(dialog_is_open(&mut cx), "{outcome:?} closed the dialog");
    let subject = cx.update(|_, cx| report.read(cx).subject.read(cx).value().to_string());
    assert_eq!(subject, "Crash", "{outcome:?} lost the subject");
}

#[gpui_kit::test]
fn a_browser_hand_off_keeps_the_dialog_open(cx: &mut TestAppContext) {
    assert_hand_off_keeps_the_dialog(cx, Outcome::BrowserReady);
}

#[gpui_kit::test]
fn a_browser_that_will_not_open_keeps_the_dialog_open(cx: &mut TestAppContext) {
    assert_hand_off_keeps_the_dialog(cx, Outcome::BrowserFailed);
}

#[gpui_kit::test]
fn with_no_window_the_item_does_nothing(cx: &mut TestAppContext) {
    let stub = Stub::answering(Outcome::BrowserReady);
    cx.update(|cx| {
          gpui_kit::init(cx);
          register_report_bug_action_with(stub.services(), cx);
          cx.dispatch_action(&ReportBug);
      });
    cx.run_until_parked();

    assert!(cx.update(|cx| open_report(cx)).is_none());
}
