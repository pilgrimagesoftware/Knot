//! Unit tests for [`super::issues_message`]: which state the Issues tab
//! shows, one test per message.

use knot_forge::ForgeAvailability;

use super::{IssuesFacts, IssuesMessage, issues_message};

fn ready() -> IssuesFacts<'static> {
    static READY: ForgeAvailability = ForgeAvailability::Ready;
    IssuesFacts { forge:          Some(&READY),
                  repos_resolved: true,
                  github_repos:   2,
                  answered:       2,
                  total_issues:   5,
                  shown_issues:   5,
                  any_failed:     false, }
}

#[test]
fn rows_draw_with_no_message() {
    assert_eq!(issues_message(&ready()), None);
}

#[test]
fn the_tool_states_say_which_fix_applies() {
    let missing = ForgeAvailability::Missing;
    let signed_out = ForgeAvailability::Unauthenticated;
    assert_eq!(issues_message(&IssuesFacts { forge: Some(&missing),
                                             ..ready() }),
               Some(IssuesMessage::ForgeMissing));
    assert_eq!(issues_message(&IssuesFacts { forge: Some(&signed_out),
                                             ..ready() }),
               Some(IssuesMessage::ForgeUnauthenticated));
}

#[test]
fn nothing_answered_yet_is_checking() {
    assert_eq!(issues_message(&IssuesFacts { forge: None,
                                             ..ready() }),
               Some(IssuesMessage::Checking));
    assert_eq!(issues_message(&IssuesFacts { repos_resolved: false,
                                             ..ready() }),
               Some(IssuesMessage::Checking));
    assert_eq!(issues_message(&IssuesFacts { answered: 0,
                                             ..ready() }),
               Some(IssuesMessage::Checking));
}

#[test]
fn no_github_repository_says_so() {
    assert_eq!(issues_message(&IssuesFacts { github_repos: 0,
                                             answered: 0,
                                             ..ready() }),
               Some(IssuesMessage::NoGithubRepos));
}

/// "No open issues" only once every repository answered and none failed.
#[test]
fn no_open_issues_waits_for_every_answer() {
    let none = IssuesFacts { total_issues: 0,
                             shown_issues: 0,
                             ..ready() };
    assert_eq!(issues_message(&none), Some(IssuesMessage::NoneOpen));
    assert_eq!(issues_message(&IssuesFacts { answered: 1,
                                             ..none }),
               None,
               "one repository still loading is not 'no open issues'");
    assert_eq!(issues_message(&IssuesFacts { any_failed: true,
                                             total_issues: 0,
                                             shown_issues: 0,
                                             ..ready() }),
               None,
               "a failed repository shows its failure, not 'no open issues'");
}

#[test]
fn a_search_hiding_everything_says_nothing_matches() {
    assert_eq!(issues_message(&IssuesFacts { shown_issues: 0,
                                             ..ready() }),
               Some(IssuesMessage::NoneMatch));
}
