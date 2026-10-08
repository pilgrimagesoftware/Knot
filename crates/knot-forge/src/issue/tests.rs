//! Unit tests for [`super`].

use super::{create_issue_with, create_labeled_issue_with, issue_list_with};
use crate::error::ForgeError;
use crate::repo_slug::RepoSlug;
use crate::runner::stub::StubRunner;

const REPO: &str = "pilgrimagesoftware/Knot";
const URL: &str = "https://github.com/pilgrimagesoftware/Knot/issues/1";

fn slug() -> RepoSlug {
    RepoSlug { owner: "acme".to_owned(),
               repo:  "widget".to_owned(), }
}

fn raw_issue(number: u64) -> String {
    format!(r#"{{"number":{number},"title":"Title {number}","url":"https://github.com/acme/widget/issues/{number}","labels":[{{"name":"bug"}},{{"name":"p1"}}],"author":{{"login":"octocat"}},"createdAt":"2024-01-01T00:00:00Z","updatedAt":"2024-01-{number:02}T00:00:00Z"}}"#)
}

#[test]
fn files_with_title_and_body_as_separate_arguments() {
    let runner = StubRunner::new(|args| {
        assert_eq!(args,
                   ["issue",
                    "create",
                    "--repo",
                    REPO,
                    "--title",
                    "Crash; rm -rf ~",
                    "--body",
                    "line one\n--title x"]);
        Ok(URL.to_owned())
    });

    let url = create_issue_with(&runner, REPO, "Crash; rm -rf ~", "line one\n--title x").unwrap();

    assert_eq!(url, URL);
    assert_eq!(runner.calls().len(), 1);
}

#[test]
fn the_url_is_taken_from_the_last_line() {
    let runner = StubRunner::ok(&format!("Creating issue in {REPO}\n\n{URL}"));

    assert_eq!(create_issue_with(&runner, REPO, "t", "b").unwrap(), URL);
}

#[test]
fn output_without_a_url_is_a_parse_error() {
    let runner = StubRunner::ok("something unexpected");

    assert!(matches!(create_issue_with(&runner, REPO, "t", "b"),
                     Err(ForgeError::Parse(_))));
}

#[test]
fn a_command_failure_maps_through_unchanged() {
    let runner = StubRunner::failing("HTTP 403: Resource not accessible", 1);

    match create_issue_with(&runner, REPO, "t", "b") {
        Err(ForgeError::Command { output, code, .. }) => {
            assert!(output.contains("403"));
            assert_eq!(code, 1);
        }
        other => panic!("expected Command, got {other:?}"),
    }
}

#[test]
fn a_timeout_maps_through_unchanged() {
    let runner = StubRunner::new(|args| Err(ForgeError::Timeout { command: args.join(" "), }));

    assert!(matches!(create_issue_with(&runner, REPO, "t", "b"),
                     Err(ForgeError::Timeout { .. })));
}

#[test]
fn a_missing_binary_maps_through_unchanged() {
    assert!(matches!(create_issue_with(&StubRunner::missing(), REPO, "t", "b"),
                     Err(ForgeError::Missing)));
}

#[test]
fn a_label_is_passed_as_its_own_argument() {
    let runner = StubRunner::new(|args| {
        assert_eq!(&args[args.len() - 2..], ["--label", "enhancement"]);
        Ok(URL.to_owned())
    });

    assert_eq!(create_labeled_issue_with(&runner, REPO, "t", "b", "enhancement").unwrap(),
               URL);
    assert_eq!(runner.calls().len(), 1);
}

#[test]
fn a_rejected_label_is_dropped_rather_than_the_report() {
    let runner = StubRunner::new(|args| {
        if args.contains(&"--label") {
            Err(ForgeError::Command { command: "issue create".to_owned(),
                                      output:  "could not add label".to_owned(),
                                      code:    1, })
        }
        else {
            Ok(URL.to_owned())
        }
    });

    assert_eq!(create_labeled_issue_with(&runner, REPO, "t", "b", "bug").unwrap(),
               URL);
    assert_eq!(runner.calls().len(), 2);
}

#[test]
fn a_timeout_is_not_retried() {
    let runner = StubRunner::new(|args| Err(ForgeError::Timeout { command: args.join(" "), }));

    assert!(matches!(create_labeled_issue_with(&runner, REPO, "t", "b", "bug"),
                     Err(ForgeError::Timeout { .. })));
    assert_eq!(runner.calls().len(),
               1,
               "a timeout may have filed the issue already");
}

#[test]
fn issue_list_runs_the_expected_argv() {
    let runner = StubRunner::new(|args| {
        assert_eq!(args,
                   ["issue",
                    "list",
                    "--repo",
                    "acme/widget",
                    "--state",
                    "open",
                    "--limit",
                    "11",
                    "--json",
                    "number,title,url,labels,author,createdAt,updatedAt"]);
        Ok("[]".to_owned())
    });

    let page = issue_list_with(&runner, &slug(), 10).unwrap();

    assert!(page.issues.is_empty());
    assert!(!page.truncated);
}

#[test]
fn issue_list_parses_every_field() {
    let runner = StubRunner::ok(&format!("[{}]", raw_issue(1)));

    let page = issue_list_with(&runner, &slug(), 10).unwrap();

    assert_eq!(page.issues.len(), 1);
    let issue = &page.issues[0];
    assert_eq!(issue.number, 1);
    assert_eq!(issue.title, "Title 1");
    assert_eq!(issue.url, "https://github.com/acme/widget/issues/1");
    assert_eq!(issue.labels, vec!["bug".to_owned(), "p1".to_owned()]);
    assert_eq!(issue.author.as_deref(), Some("octocat"));
    assert_eq!(issue.created_at.unix_timestamp(), 1704067200);
    assert!(!page.truncated);
}

#[test]
fn issue_list_reports_truncation_when_the_limit_plus_one_came_back() {
    let issues = (1..=11).map(raw_issue).collect::<Vec<_>>().join(",");
    let runner = StubRunner::ok(&format!("[{issues}]"));

    let page = issue_list_with(&runner, &slug(), 10).unwrap();

    assert_eq!(page.issues.len(), 10);
    assert!(page.truncated);
}

#[test]
fn issue_list_is_not_truncated_at_exactly_the_limit() {
    let issues = (1..=10).map(raw_issue).collect::<Vec<_>>().join(",");
    let runner = StubRunner::ok(&format!("[{issues}]"));

    let page = issue_list_with(&runner, &slug(), 10).unwrap();

    assert_eq!(page.issues.len(), 10);
    assert!(!page.truncated);
}

#[test]
fn issue_list_surfaces_a_failed_run() {
    let runner = StubRunner::failing("HTTP 404: Not Found", 1);

    match issue_list_with(&runner, &slug(), 10) {
        Err(ForgeError::Command { output, code, .. }) => {
            assert!(output.contains("404"));
            assert_eq!(code, 1);
        }
        other => panic!("expected Command, got {other:?}"),
    }
}

#[test]
fn issue_list_surfaces_a_missing_binary() {
    assert!(matches!(issue_list_with(&StubRunner::missing(), &slug(), 10),
                     Err(ForgeError::Missing)));
}

#[test]
fn issue_list_rejects_an_unparseable_timestamp() {
    let bad = r#"[{"number":1,"title":"t","url":"https://github.com/acme/widget/issues/1","labels":[],"author":null,"createdAt":"not-a-date","updatedAt":"2024-01-01T00:00:00Z"}]"#;
    let runner = StubRunner::ok(bad);

    assert!(matches!(issue_list_with(&runner, &slug(), 10),
                     Err(ForgeError::Parse(_))));
}
