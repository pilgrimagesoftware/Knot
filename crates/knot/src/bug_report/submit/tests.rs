//! Unit tests for [`super`].

use std::cell::RefCell;

use knot_core::consts::KNOT_REPO;
use knot_forge::{ForgeAvailability, ForgeError, ForgeRunner};

use super::*;
use crate::bug_report::kind::IssueKind;
use crate::bug_report::logs::LogKind;

/// Answers every `gh` call with one result, recording the arguments.
struct Gh {
    answer: fn() -> knot_forge::Result<String>,
    calls:  RefCell<Vec<Vec<String>>>,
}

impl Gh {
    fn new(answer: fn() -> knot_forge::Result<String>) -> Self {
        Self { answer,
               calls: RefCell::new(Vec::new()) }
    }
}

impl ForgeRunner for Gh {
    fn run(&self, args: &[&str]) -> knot_forge::Result<String> {
        self.calls
            .borrow_mut()
            .push(args.iter().map(|arg| (*arg).to_owned()).collect());
        (self.answer)()
    }
}

const FILED: &str = "https://github.com/pilgrimagesoftware/Knot/issues/7";

fn report() -> Report {
    Report { kind:        IssueKind::Bug,
             subject:     "  Crash on launch ".to_owned(),
             description: "It crashed.\nEvery time.".to_owned(),
             diagnostics: "App: Knot 1.0.0 (2026-09-25, abc)".to_owned(),
             attachments: Vec::new(),
             screenshots: Vec::new(), }
}

fn report_with_log() -> Report {
    Report { attachments: vec![Attachment { kind: LogKind::Mcp,
                                            path: Some("/tmp/Knot/knot-mcp.jsonl".into()),
                                            tail: Some("{\"subject\":\"lifecycle\"}".to_owned()), }],
             ..report() }
}

fn never_opens(_: &str) -> bool {
    panic!("the browser must not open when gh can file the issue")
}

fn never_reveals(_: &std::path::Path) -> bool {
    panic!("nothing is shown in Finder for a report without screenshots")
}

fn report_with_screenshots() -> Report {
    Report { screenshots: vec!["/Users/me/Desktop/crash.png".into(),
                               "/Users/me/Desktop/after.png".into()],
             ..report() }
}

#[test]
fn a_ready_forge_files_the_issue() {
    let gh = Gh::new(|| Ok(FILED.to_owned()));

    let outcome = submit(&report(),
                         &ForgeAvailability::Ready,
                         &gh,
                         never_opens,
                         never_reveals);

    assert_eq!(outcome, Outcome::Filed(FILED.to_owned()));
    let calls = gh.calls.borrow();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0],
               ["issue",
                "create",
                "--repo",
                KNOT_REPO,
                "--title",
                "Crash on launch",
                "--body",
                &issue_body(&report()),
                "--label",
                "bug"]);
}

#[test]
fn a_failed_filing_says_why_and_can_be_retried() {
    let gh = Gh::new(|| {
        Err(ForgeError::Command { command: "issue create".to_owned(),
                                  output:  "HTTP 403".to_owned(),
                                  code:    1, })
    });

    let first = submit(&report(),
                       &ForgeAvailability::Ready,
                       &gh,
                       never_opens,
                       never_reveals);
    let second = submit(&report(),
                        &ForgeAvailability::Ready,
                        &gh,
                        never_opens,
                        never_reveals);

    assert!(matches!(&first, Outcome::FileFailed(why) if why.contains("HTTP 403")),
            "{first:?}");
    assert_eq!(first, second);
    // Two calls per attempt: labeled, then again without the label, which
    // is how a filing refused over the label still goes through.
    assert_eq!(gh.calls.borrow().len(), 4, "the retry did not reach gh");
}

#[test]
fn a_forge_that_is_not_ready_opens_the_compose_page() {
    for forge in [ForgeAvailability::Missing,
                  ForgeAvailability::Unauthenticated,
                  ForgeAvailability::Failed("no such host".to_owned())]
    {
        let gh = Gh::new(|| panic!("gh must not run when it is not ready"));
        let opened = RefCell::new(None);

        let outcome = submit(&report(),
                             &forge,
                             &gh,
                             |url| {
                                 *opened.borrow_mut() = Some(url.to_owned());
                                 true
                             },
                             never_reveals);

        assert_eq!(outcome, Outcome::BrowserReady, "{forge:?}");
        assert_eq!(opened.into_inner(),
                   Some(compose_url(&report())),
                   "{forge:?}");
    }
}

#[test]
fn a_browser_that_will_not_open_is_reported() {
    let gh = Gh::new(|| panic!("gh must not run when it is not ready"));

    assert_eq!(submit(&report(),
                      &ForgeAvailability::Missing,
                      &gh,
                      |_| false,
                      never_reveals),
               Outcome::BrowserFailed);
}

#[test]
fn the_body_carries_the_description_then_the_diagnostics() {
    let body = issue_body(&report());

    let description = body.find("It crashed.\nEvery time.")
                          .expect("description missing");
    let diagnostics = body.find("App: Knot 1.0.0").expect("diagnostics missing");
    assert!(description < diagnostics, "{body}");
    assert!(body.contains(&knot_core::l10n::t("bug_report.diagnostics_label")));
}

#[test]
fn the_compose_url_targets_the_same_repo_as_gh() {
    let url = compose_url(&report());

    assert!(url.starts_with(&format!("https://github.com/{KNOT_REPO}/issues/new?")),
            "{url}");
    assert!(url.contains("title=Crash%20on%20launch&"), "{url}");
    assert!(url.contains(&format!("body={}", percent_encode(&issue_body(&report())))),
            "{url}");
}

#[test]
fn an_attached_log_follows_the_diagnostics() {
    let body = issue_body(&report_with_log());

    let diagnostics = body.find("App: Knot 1.0.0").expect("diagnostics missing");
    let log = body.find("\"lifecycle\"")
                  .expect("the log was not attached");
    assert!(diagnostics < log, "{body}");
}

#[test]
fn no_logs_heading_without_a_log() {
    assert!(!issue_body(&report()).contains(&knot_core::l10n::t("bug_report.logs.label")));
}

#[test]
fn the_compose_url_names_the_log_rather_than_carrying_it() {
    let url = compose_url(&report_with_log());

    assert!(!url.contains("lifecycle"), "{url}");
    assert!(url.contains(&percent_encode("/tmp/Knot/knot-mcp.jsonl")),
            "{url}");
}

#[test]
fn encoding_keeps_unreserved_characters() {
    assert_eq!(percent_encode("Aa0-._~"), "Aa0-._~");
}

#[test]
fn encoding_escapes_spaces_slashes_and_query_syntax() {
    assert_eq!(percent_encode("a b/c&d=e#f?g+h\n"),
               "a%20b%2Fc%26d%3De%23f%3Fg%2Bh%0A");
}

#[test]
fn encoding_escapes_unicode_as_utf8_bytes() {
    assert_eq!(percent_encode("é…"), "%C3%A9%E2%80%A6");
}

#[test]
fn a_feature_request_is_filed_as_an_enhancement() {
    let gh = Gh::new(|| Ok(FILED.to_owned()));
    let request = Report { kind: IssueKind::FeatureRequest,
                           ..report() };

    submit(&request,
           &ForgeAvailability::Ready,
           &gh,
           never_opens,
           never_reveals);

    let calls = gh.calls.borrow();
    assert_eq!(&calls[0][calls[0].len() - 2..], ["--label", "enhancement"]);
}

#[test]
fn the_compose_url_carries_the_label() {
    let request = Report { kind: IssueKind::FeatureRequest,
                           ..report() };
    assert!(compose_url(&request).ends_with("&labels=enhancement"));
}

/// A filed report with screenshots names them in the body, then opens the
/// filed issue and shows each file, so the user can drag them in (#566).
#[test]
fn a_filed_report_with_screenshots_opens_the_issue_and_shows_them() {
    let gh = Gh::new(|| Ok(FILED.to_owned()));
    let opened = RefCell::new(Vec::new());
    let revealed = RefCell::new(Vec::new());

    let outcome = submit(&report_with_screenshots(),
                         &ForgeAvailability::Ready,
                         &gh,
                         |url| {
                             opened.borrow_mut().push(url.to_owned());
                             true
                         },
                         |path| {
                             revealed.borrow_mut().push(path.to_path_buf());
                             true
                         });

    assert_eq!(outcome, Outcome::Filed(FILED.to_owned()));
    assert_eq!(opened.into_inner(), [FILED], "the filed issue's page opens");
    assert_eq!(revealed.into_inner(), report_with_screenshots().screenshots);
    let body = &gh.calls.borrow()[0];
    let body = &body[body.iter().position(|arg| arg == "--body").unwrap() + 1];
    assert!(body.contains("`crash.png`") && body.contains("`after.png`"),
            "{body}");
}

/// The browser path names them on the compose page and shows them too.
#[test]
fn the_browser_path_names_and_shows_the_screenshots() {
    let gh = Gh::new(|| panic!("gh must not run when it is not ready"));
    let revealed = RefCell::new(Vec::new());

    let outcome = submit(&report_with_screenshots(),
                         &ForgeAvailability::Missing,
                         &gh,
                         |_| true,
                         |path| {
                             revealed.borrow_mut().push(path.to_path_buf());
                             true
                         });

    assert_eq!(outcome, Outcome::BrowserReady);
    assert_eq!(revealed.into_inner().len(), 2);
    assert!(compose_body(&report_with_screenshots()).contains("`crash.png`"));
}

/// A browser that never opened leaves the report in the dialog to retry, so
/// nothing is shown in Finder for a page that is not there.
#[test]
fn a_failed_browser_shows_nothing() {
    let gh = Gh::new(|| panic!("gh must not run when it is not ready"));

    assert_eq!(submit(&report_with_screenshots(),
                      &ForgeAvailability::Missing,
                      &gh,
                      |_| false,
                      never_reveals),
               Outcome::BrowserFailed);
}

/// A failed filing has no issue to drag anything onto.
#[test]
fn a_failed_filing_shows_nothing() {
    let gh = Gh::new(|| {
        Err(ForgeError::Command { command: "issue create".to_owned(),
                                  output:  "HTTP 403".to_owned(),
                                  code:    1, })
    });

    let outcome = submit(&report_with_screenshots(),
                         &ForgeAvailability::Ready,
                         &gh,
                         never_opens,
                         never_reveals);

    assert!(matches!(outcome, Outcome::FileFailed(_)), "{outcome:?}");
}

#[test]
fn a_report_without_screenshots_has_no_section() {
    assert!(!issue_body(&report()).contains(&knot_core::l10n::t("bug_report.screenshots.label")));
}
