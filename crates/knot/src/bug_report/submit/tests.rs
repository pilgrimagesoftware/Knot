//! Unit tests for [`super`].

use std::cell::RefCell;

use knot_core::consts::KNOT_REPO;
use knot_forge::{ForgeAvailability, ForgeError, ForgeRunner};

use super::*;

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
    Report { subject:     "  Crash on launch ".to_owned(),
             description: "It crashed.\nEvery time.".to_owned(),
             diagnostics: "App: Knot 1.0.0 (2026-09-25, abc)".to_owned(), }
}

fn never_opens(_: &str) -> bool {
    panic!("the browser must not open when gh can file the issue")
}

#[test]
fn a_ready_forge_files_the_issue() {
    let gh = Gh::new(|| Ok(FILED.to_owned()));

    let outcome = submit(&report(), &ForgeAvailability::Ready, &gh, never_opens);

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
                &issue_body(&report())]);
}

#[test]
fn a_failed_filing_says_why_and_can_be_retried() {
    let gh = Gh::new(|| {
        Err(ForgeError::Command { command: "issue create".to_owned(),
                                  output:  "HTTP 403".to_owned(),
                                  code:    1, })
    });

    let first = submit(&report(), &ForgeAvailability::Ready, &gh, never_opens);
    let second = submit(&report(), &ForgeAvailability::Ready, &gh, never_opens);

    assert!(matches!(&first, Outcome::FileFailed(why) if why.contains("HTTP 403")),
            "{first:?}");
    assert_eq!(first, second);
    assert_eq!(gh.calls.borrow().len(), 2, "the retry did not reach gh");
}

#[test]
fn a_forge_that_is_not_ready_opens_the_compose_page() {
    for forge in [ForgeAvailability::Missing,
                  ForgeAvailability::Unauthenticated,
                  ForgeAvailability::Failed("no such host".to_owned())]
    {
        let gh = Gh::new(|| panic!("gh must not run when it is not ready"));
        let opened = RefCell::new(None);

        let outcome = submit(&report(), &forge, &gh, |url| {
            *opened.borrow_mut() = Some(url.to_owned());
            true
        });

        assert_eq!(outcome, Outcome::BrowserReady, "{forge:?}");
        assert_eq!(opened.into_inner(),
                   Some(compose_url(&report())),
                   "{forge:?}");
    }
}

#[test]
fn a_browser_that_will_not_open_is_reported() {
    let gh = Gh::new(|| panic!("gh must not run when it is not ready"));

    assert_eq!(submit(&report(), &ForgeAvailability::Missing, &gh, |_| false),
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
