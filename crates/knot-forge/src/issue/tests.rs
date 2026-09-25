//! Unit tests for [`super`].

use super::create_issue_with;
use crate::error::ForgeError;
use crate::runner::stub::StubRunner;

const REPO: &str = "pilgrimagesoftware/Knot";
const URL: &str = "https://github.com/pilgrimagesoftware/Knot/issues/1";

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
