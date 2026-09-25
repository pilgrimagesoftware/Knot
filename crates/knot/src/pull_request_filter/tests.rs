//! Unit tests for [`super`]: what the search, the filters and the sort leave
//! on screen, one test per spec scenario where the scenario is decidable
//! without a window.

use std::collections::BTreeSet;

use knot_forge::{Mergeability, PullRequestState, PullRequestStatus};
use uuid::Uuid;

use super::{
    PullRequestGroup, PullRequestRow, PullRequestSort, RowCategory, ViewFilter, apply,
    attention_rank, category_counts, copied_urls, listed_agents, repo_and_number, shown_rows,
};
use crate::pull_request_state::PullRequestLookup;

fn url(repo: &str, number: u64) -> String {
    format!("https://github.com/{repo}/pull/{number}")
}

fn known(status: PullRequestStatus, mergeable: Mergeability, title: &str, number: u64)
         -> Option<PullRequestLookup> {
    Some(PullRequestLookup::Known(PullRequestState { number: Some(number),
                                                     title: Some(title.to_string()),
                                                     status,
                                                     checks: None,
                                                     mergeable,
                                                     merged_at: None }))
}

fn row(url: String, lookup: Option<PullRequestLookup>, first_seen: i64) -> PullRequestRow {
    PullRequestRow { url,
                     lookup,
                     first_seen }
}

fn open(repo: &str, number: u64, title: &str, first_seen: i64) -> PullRequestRow {
    row(url(repo, number),
        known(PullRequestStatus::Open,
              Mergeability::Mergeable,
              title,
              number),
        first_seen)
}

fn with_status(repo: &str, number: u64, status: PullRequestStatus, first_seen: i64)
               -> PullRequestRow {
    row(url(repo, number),
        known(status, Mergeability::Mergeable, "untitled", number),
        first_seen)
}

fn group(agent_ids: Vec<Uuid>, agents: &str, rows: Vec<PullRequestRow>) -> PullRequestGroup {
    PullRequestGroup { agent_ids,
                       agents: agents.to_string(),
                       rows }
}

fn urls(groups: &[PullRequestGroup]) -> Vec<String> {
    groups.iter()
          .flat_map(|group| group.rows.iter().map(|row| row.url.clone()))
          .collect()
}

fn search(text: &str) -> ViewFilter {
    ViewFilter { search: text.to_string(),
                 ..ViewFilter::default() }
}

fn statuses(categories: &[RowCategory]) -> ViewFilter {
    ViewFilter { statuses: categories.iter().copied().collect(),
                 ..ViewFilter::default() }
}

#[test]
fn no_filter_shows_everything_newest_first() {
    let agent = Uuid::new_v4();
    let groups = vec![group(vec![agent],
                            "Ada",
                            vec![open("acme/widget", 1, "old", 100),
                                 open("acme/widget", 2, "new", 200)])];

    let shown = apply(groups, &ViewFilter::default(), PullRequestSort::default());

    assert_eq!(urls(&shown),
               vec![url("acme/widget", 2), url("acme/widget", 1)]);
}

#[test]
fn searching_by_title_ignores_case() {
    let groups = vec![group(vec![Uuid::new_v4()],
                            "Ada",
                            vec![open("acme/widget", 1, "Fix login redirect", 1),
                                 open("acme/widget", 2, "Add billing export", 2)])];

    let shown = apply(groups, &search("LOGIN"), PullRequestSort::default());

    assert_eq!(urls(&shown), vec![url("acme/widget", 1)]);
}

#[test]
fn searching_by_number() {
    let groups = vec![group(vec![Uuid::new_v4()],
                            "Ada",
                            vec![open("acme/widget", 42, "a", 1),
                                 open("acme/widget", 7, "b", 2)])];

    let shown = apply(groups, &search("#42"), PullRequestSort::default());

    assert_eq!(urls(&shown), vec![url("acme/widget", 42)]);
}

#[test]
fn searching_by_repository() {
    let groups = vec![group(vec![Uuid::new_v4()],
                            "Ada",
                            vec![open("acme/widget", 1, "a", 1),
                                 open("acme/gadget", 2, "b", 2)])];

    let shown = apply(groups, &search("acme/widget"), PullRequestSort::default());

    assert_eq!(urls(&shown), vec![url("acme/widget", 1)]);
}

#[test]
fn searching_by_agent_includes_shared_rows() {
    let (ada, bo) = (Uuid::new_v4(), Uuid::new_v4());
    let groups = vec![group(vec![ada, bo], "Ada, Bo", vec![open("acme/a", 1, "x", 1)]),
                      group(vec![ada], "Ada", vec![open("acme/b", 2, "y", 2)]),
                      group(vec![bo], "Bo", vec![open("acme/c", 3, "z", 3)])];

    let shown = apply(groups, &search("ada"), PullRequestSort::default());

    assert_eq!(urls(&shown), vec![url("acme/a", 1), url("acme/b", 2)]);
}

#[test]
fn a_row_without_state_still_matches_on_its_url() {
    let groups = vec![group(vec![Uuid::new_v4()],
                            "Ada",
                            vec![row(url("acme/widget", 5), None, 1)])];

    let by_repo = apply(groups.clone(),
                        &search("widget"),
                        PullRequestSort::default());
    let by_number = apply(groups, &search("#5"), PullRequestSort::default());

    assert_eq!(urls(&by_repo), vec![url("acme/widget", 5)]);
    assert_eq!(urls(&by_number), vec![url("acme/widget", 5)]);
}

#[test]
fn whitespace_only_search_is_empty() {
    let groups = vec![group(vec![Uuid::new_v4()],
                            "Ada",
                            vec![open("acme/widget", 1, "a", 1)])];

    let filter = search("   ");
    let shown = apply(groups, &filter, PullRequestSort::default());

    assert_eq!(shown.len(), 1);
    assert!(!filter.is_active());
}

#[test]
fn the_open_toggle_includes_drafts() {
    let groups = vec![group(vec![Uuid::new_v4()],
                            "Ada",
                            vec![with_status("acme/w", 1, PullRequestStatus::Open, 1),
                                 with_status("acme/w", 2, PullRequestStatus::Draft, 2),
                                 with_status("acme/w", 3, PullRequestStatus::Merged, 3)])];

    let shown = apply(groups,
                      &statuses(&[RowCategory::Open]),
                      PullRequestSort::default());

    assert_eq!(urls(&shown), vec![url("acme/w", 2), url("acme/w", 1)]);
}

#[test]
fn two_toggles_show_either_status() {
    let groups = vec![group(vec![Uuid::new_v4()],
                            "Ada",
                            vec![with_status("acme/w", 1, PullRequestStatus::Closed, 1),
                                 row(url("acme/w", 2), Some(PullRequestLookup::NotFound), 2),
                                 with_status("acme/w", 3, PullRequestStatus::Open, 3)])];

    let shown = apply(groups,
                      &statuses(&[RowCategory::Closed, RowCategory::NotFound]),
                      PullRequestSort::default());

    assert_eq!(urls(&shown), vec![url("acme/w", 2), url("acme/w", 1)]);
}

#[test]
fn a_failed_fetch_is_pending() {
    assert_eq!(RowCategory::of(Some(&PullRequestLookup::Failed)),
               RowCategory::Pending);
    assert_eq!(RowCategory::of(None), RowCategory::Pending);
}

#[test]
fn a_toggles_count_follows_the_search_but_not_the_status_filter() {
    let groups = vec![group(vec![Uuid::new_v4()],
                            "Ada",
                            vec![with_status("acme/widget", 1, PullRequestStatus::Merged, 1),
                                 with_status("acme/gadget", 2, PullRequestStatus::Merged, 2),
                                 with_status("acme/gadget", 3, PullRequestStatus::Merged, 3),
                                 with_status("acme/widget", 4, PullRequestStatus::Open, 4)])];
    let mut filter = search("acme/widget");
    filter.statuses = BTreeSet::from([RowCategory::Open]);

    let counts = category_counts(&groups, &filter);

    assert_eq!(counts.get(&RowCategory::Merged), Some(&1));
    assert_eq!(counts.get(&RowCategory::Open), Some(&1));
}

#[test]
fn one_agents_pull_requests_keep_the_shared_heading() {
    let (ada, bo) = (Uuid::new_v4(), Uuid::new_v4());
    let groups = vec![group(vec![ada, bo], "Ada, Bo", vec![open("acme/a", 1, "x", 1)]),
                      group(vec![ada], "Ada", vec![open("acme/b", 2, "y", 2)]),
                      group(vec![bo], "Bo", vec![open("acme/c", 3, "z", 3)])];
    let filter = ViewFilter { agent: Some(ada),
                              ..ViewFilter::default() };

    let shown = apply(groups, &filter, PullRequestSort::default());

    assert_eq!(shown.len(), 2);
    assert_eq!(shown[0].agents, "Ada, Bo");
    assert_eq!(urls(&shown), vec![url("acme/a", 1), url("acme/b", 2)]);
}

#[test]
fn listed_agents_names_every_heading_agent() {
    let (ada, bo) = (Uuid::new_v4(), Uuid::new_v4());
    let groups = vec![group(vec![ada, bo], "Ada, Bo", vec![open("acme/a", 1, "x", 1)])];

    assert_eq!(listed_agents(&groups), BTreeSet::from([ada, bo]));
}

#[test]
fn oldest_first() {
    let groups = vec![group(vec![Uuid::new_v4()],
                            "Ada",
                            vec![open("acme/w", 2, "tuesday", 200),
                                 open("acme/w", 1, "monday", 100)])];

    let shown = apply(groups, &ViewFilter::default(), PullRequestSort::OldestFirst);

    assert_eq!(urls(&shown), vec![url("acme/w", 1), url("acme/w", 2)]);
}

#[test]
fn needs_attention_puts_conflicts_first_and_merged_last() {
    let conflicting = row(url("acme/w", 1),
                          known(PullRequestStatus::Open, Mergeability::Conflicting, "c", 1),
                          1);
    let groups = vec![group(vec![Uuid::new_v4()],
                            "Ada",
                            vec![with_status("acme/w", 3, PullRequestStatus::Merged, 3),
                                 open("acme/w", 2, "m", 2),
                                 conflicting])];

    let shown = apply(groups,
                      &ViewFilter::default(),
                      PullRequestSort::NeedsAttention);

    assert_eq!(urls(&shown),
               vec![url("acme/w", 1), url("acme/w", 2), url("acme/w", 3)]);
}

/// The ranks follow the row colours' precedence, and every row that is not
/// open ranks after every one that is.
#[test]
fn attention_ranks_follow_the_colour_precedence() {
    let rank = |mergeable| {
        attention_rank(&row(url("a/b", 1),
                            known(PullRequestStatus::Open, mergeable, "t", 1),
                            0))
    };
    let ranks = [rank(Mergeability::Conflicting),
                 rank(Mergeability::Behind),
                 rank(Mergeability::ChecksRunning),
                 rank(Mergeability::Unknown),
                 rank(Mergeability::Mergeable),
                 attention_rank(&row(url("a/b", 1), None, 0)),
                 attention_rank(&row(url("a/b", 1), Some(PullRequestLookup::NotFound), 0)),
                 attention_rank(&with_status("a/b", 1, PullRequestStatus::Closed, 0)),
                 attention_rank(&with_status("a/b", 1, PullRequestStatus::Merged, 0))];

    assert!(ranks.windows(2).all(|pair| pair[0] < pair[1]), "{ranks:?}");
    assert_eq!(rank(Mergeability::Blocked), rank(Mergeability::Conflicting));
}

#[test]
fn repository_order_then_number() {
    let groups = vec![group(vec![Uuid::new_v4()],
                            "Ada",
                            vec![open("acme/widget", 9, "a", 3),
                                 open("acme/gadget", 12, "b", 2),
                                 open("acme/widget", 3, "c", 1)])];

    let shown = apply(groups, &ViewFilter::default(), PullRequestSort::Repository);

    assert_eq!(urls(&shown),
               vec![url("acme/gadget", 12),
                    url("acme/widget", 3),
                    url("acme/widget", 9)]);
}

#[test]
fn sorting_never_reorders_groups() {
    let (ada, bo) = (Uuid::new_v4(), Uuid::new_v4());
    let groups = vec![group(vec![ada, bo], "Ada, Bo", vec![open("zzz/z", 1, "x", 1)]),
                      group(vec![ada], "Ada", vec![open("aaa/a", 2, "y", 2)])];

    for sort in PullRequestSort::ALL {
        let shown = apply(groups.clone(), &ViewFilter::default(), sort);

        assert_eq!(shown[0].agents, "Ada, Bo", "{sort:?}");
    }
}

#[test]
fn nothing_matching_leaves_no_groups() {
    let groups = vec![group(vec![Uuid::new_v4()], "Ada", vec![open("acme/w", 1, "a", 1)])];

    let shown = apply(groups.clone(),
                      &search("no such thing"),
                      PullRequestSort::default());

    assert!(shown.is_empty());
    assert!(!groups.is_empty(),
            "an empty result is not an empty workspace");
}

#[test]
fn search_status_and_agent_combine() {
    let (ada, bo) = (Uuid::new_v4(), Uuid::new_v4());
    let groups = vec![group(vec![ada],
                            "Ada",
                            vec![with_status("acme/widget", 1, PullRequestStatus::Merged, 1),
                                 with_status("acme/widget", 2, PullRequestStatus::Open, 2),
                                 with_status("acme/gadget", 3, PullRequestStatus::Merged, 3)]),
                      group(vec![bo],
                            "Bo",
                            vec![with_status("acme/widget", 4, PullRequestStatus::Merged, 4)])];
    let filter = ViewFilter { search:   "widget".to_string(),
                              statuses: BTreeSet::from([RowCategory::Merged]),
                              agent:    Some(ada), };

    let shown = apply(groups, &filter, PullRequestSort::default());

    assert_eq!(urls(&shown), vec![url("acme/widget", 1)]);
}

#[test]
fn bulk_actions_take_only_the_shown_rows_of_a_status() {
    let (ada, bo) = (Uuid::new_v4(), Uuid::new_v4());
    let shown = vec![group(vec![ada, bo],
                           "Ada, Bo",
                           vec![with_status("acme/w", 1, PullRequestStatus::Merged, 1)]),
                     group(vec![ada],
                           "Ada",
                           vec![with_status("acme/w", 2, PullRequestStatus::Open, 2),
                                with_status("acme/w", 3, PullRequestStatus::Merged, 3)])];

    let merged = shown_rows(&shown, Some(RowCategory::Merged));

    assert_eq!(merged,
               vec![(vec![ada, bo], url("acme/w", 1)),
                    (vec![ada], url("acme/w", 3))]);
    assert_eq!(shown_rows(&shown, None).len(), 3);
}

#[test]
fn copied_urls_are_one_per_line_in_shown_order() {
    let shown = vec![group(vec![Uuid::new_v4()],
                           "Ada",
                           vec![open("acme/w", 2, "a", 2), open("acme/w", 1, "b", 1)])];

    assert_eq!(copied_urls(&shown),
               format!("{}\n{}", url("acme/w", 2), url("acme/w", 1)));
}

#[test]
fn repository_and_number_parse_on_any_host() {
    assert_eq!(repo_and_number("https://ghe.example.com/acme/widget/pull/42"),
               Some(("acme/widget".to_string(), 42)));
    assert_eq!(repo_and_number("https://github.com/acme/widget/issues/42"),
               None);
}

#[test]
fn every_label_resolves() {
    for category in RowCategory::ALL {
        let label = category.to_string();
        assert!(!label.starts_with("pull_requests."),
                "{category:?}: {label}");
    }
    for sort in PullRequestSort::ALL {
        let label = sort.to_string();
        assert!(!label.starts_with("pull_requests."), "{sort:?}: {label}");
    }
}
