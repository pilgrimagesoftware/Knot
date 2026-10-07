//! Unit tests for [`super`]: one test per spec scenario under "The user can
//! search, filter and sort the Issues tab" and "The Issues tab lists issues
//! by repository", decidable without a window.
//!
//! This module exercises code nothing else calls yet - see the `UNWIRED`
//! markers in `super` - so passing here is not coverage of a wired feature.

use knot_forge::{Issue, RepoSlug};
use time::OffsetDateTime;
use time::macros::datetime;

use super::{IssueFilter, IssueGroup, IssueSort, apply};

fn slug(owner: &str, repo: &str) -> RepoSlug {
    RepoSlug { owner: owner.to_owned(),
               repo:  repo.to_owned(), }
}

fn issue(number: u64, title: &str, labels: &[&str], created_at: OffsetDateTime,
         updated_at: OffsetDateTime)
         -> Issue {
    Issue { number,
            title: title.to_owned(),
            url: format!("https://github.com/acme/widget/issues/{number}"),
            labels: labels.iter().map(|label| (*label).to_owned()).collect(),
            author: Some("octocat".to_owned()),
            created_at,
            updated_at }
}

fn group(owner: &str, repo: &str, issues: Vec<Issue>) -> IssueGroup {
    IssueGroup { slug: slug(owner, repo),
                 issues }
}

fn numbers(groups: &[IssueGroup]) -> Vec<u64> {
    groups.iter()
          .flat_map(|group| group.issues.iter().map(|issue| issue.number))
          .collect()
}

fn slugs(groups: &[IssueGroup]) -> Vec<String> {
    groups.iter().map(|group| group.slug.to_string()).collect()
}

const T1: OffsetDateTime = datetime!(2024-01-01 00:00 UTC);
const T2: OffsetDateTime = datetime!(2024-02-01 00:00 UTC);
const T3: OffsetDateTime = datetime!(2024-03-01 00:00 UTC);

#[test]
fn searching_by_label_is_case_insensitive() {
    let groups = vec![group("acme",
                            "widget",
                            vec![issue(1, "first", &["bug"], T1, T1),
                                 issue(2, "second", &["enhancement"], T1, T1)])];

    let shown = apply(groups,
                      &IssueFilter { search: "BUG".to_owned(),
                                     repo:   None, },
                      IssueSort::default());

    assert_eq!(numbers(&shown), vec![1]);
}

#[test]
fn searching_matches_title_number_repo_and_url() {
    let base = vec![group("acme",
                          "widget",
                          vec![issue(42, "Crash on launch", &[], T1, T1)])];

    for needle in ["crash", "#42", "acme/widget", "issues/42"] {
        let shown = apply(base.clone(),
                          &IssueFilter { search: needle.to_owned(),
                                         repo:   None, },
                          IssueSort::default());
        assert_eq!(numbers(&shown), vec![42], "needle {needle:?} should match");
    }
}

#[test]
fn search_is_trimmed() {
    let groups = vec![group("acme", "widget", vec![issue(1, "Crash", &[], T1, T1)])];

    let shown = apply(groups,
                      &IssueFilter { search: "  crash  ".to_owned(),
                                     repo:   None, },
                      IssueSort::default());

    assert_eq!(numbers(&shown), vec![1]);
}

#[test]
fn filtering_to_one_repository_hides_the_rest() {
    let groups = vec![group("acme", "widget", vec![issue(1, "a", &[], T1, T1)]),
                      group("acme", "gadget", vec![issue(2, "b", &[], T1, T1)])];

    let shown = apply(groups,
                      &IssueFilter { search: String::new(),
                                     repo:   Some(slug("acme", "widget")), },
                      IssueSort::default());

    assert_eq!(slugs(&shown), vec!["acme/widget"]);
}

#[test]
fn groups_are_ordered_by_owner_and_repo() {
    let groups = vec![group("acme", "widget", vec![issue(1, "a", &[], T1, T1)]),
                      group("acme", "gadget", vec![issue(2, "b", &[], T1, T1)])];

    let shown = apply(groups, &IssueFilter::default(), IssueSort::default());

    assert_eq!(slugs(&shown), vec!["acme/gadget", "acme/widget"]);
}

#[test]
fn a_search_matching_nothing_leaves_no_groups() {
    let groups = vec![group("acme", "widget", vec![issue(1, "a", &[], T1, T1)])];

    let shown = apply(groups,
                      &IssueFilter { search: "nothing matches this".to_owned(),
                                     repo:   None, },
                      IssueSort::default());

    assert!(shown.is_empty());
}

#[test]
fn a_repository_with_no_open_issues_is_dropped_not_shown_empty() {
    let groups = vec![group("acme", "widget", vec![])];

    let shown = apply(groups, &IssueFilter::default(), IssueSort::default());

    assert!(shown.is_empty());
}

#[test]
fn sort_by_number_is_ascending() {
    let groups = vec![group("acme",
                            "widget",
                            vec![issue(12, "a", &[], T1, T1),
                                 issue(3, "b", &[], T1, T1),
                                 issue(7, "c", &[], T1, T1)])];

    let shown = apply(groups, &IssueFilter::default(), IssueSort::Number);

    assert_eq!(numbers(&shown), vec![3, 7, 12]);
}

#[test]
fn sort_recently_updated_first_is_the_default() {
    assert_eq!(IssueSort::default(), IssueSort::RecentlyUpdated);

    let groups = vec![group("acme",
                            "widget",
                            vec![issue(1, "a", &[], T1, T1),
                                 issue(2, "b", &[], T1, T3),
                                 issue(3, "c", &[], T1, T2)])];

    let shown = apply(groups, &IssueFilter::default(), IssueSort::RecentlyUpdated);

    assert_eq!(numbers(&shown), vec![2, 3, 1]);
}

#[test]
fn sort_newest_first_orders_by_created_at_descending() {
    let groups = vec![group("acme",
                            "widget",
                            vec![issue(1, "a", &[], T1, T1),
                                 issue(2, "b", &[], T3, T1),
                                 issue(3, "c", &[], T2, T1)])];

    let shown = apply(groups, &IssueFilter::default(), IssueSort::Newest);

    assert_eq!(numbers(&shown), vec![2, 3, 1]);
}

#[test]
fn sort_oldest_first_orders_by_created_at_ascending() {
    let groups = vec![group("acme",
                            "widget",
                            vec![issue(1, "a", &[], T2, T1),
                                 issue(2, "b", &[], T3, T1),
                                 issue(3, "c", &[], T1, T1)])];

    let shown = apply(groups, &IssueFilter::default(), IssueSort::Oldest);

    assert_eq!(numbers(&shown), vec![3, 1, 2]);
}

#[test]
fn reset_missing_repo_clears_a_repository_that_is_gone() {
    let mut filter = IssueFilter { search: String::new(),
                                   repo:   Some(slug("acme", "widget")), };

    filter.reset_missing_repo(&[slug("acme", "gadget")]);

    assert_eq!(filter.repo, None);
}

#[test]
fn reset_missing_repo_keeps_a_repository_that_is_still_there() {
    let mut filter = IssueFilter { search: String::new(),
                                   repo:   Some(slug("acme", "widget")), };

    filter.reset_missing_repo(&[slug("acme", "widget"), slug("acme", "gadget")]);

    assert_eq!(filter.repo, Some(slug("acme", "widget")));
}
