//! Search and repository filtering for the OpenSpec tab's un-archived change
//! rows.
//!
//! Contract: "The user can search and filter the OpenSpec tab" in the
//! `workspace-openspec-changes` capability spec under `openspec/specs/`.
//!
//! Pure over rows already grouped by repository heading, the same split as
//! `pull_request_filter`: [`super::scan`] and [`super::merge`] decide what is
//! found, this decides what the search field and the repository picker leave
//! on screen.

use super::ChangeEntry;

/// Whether `entry`, grouped under `heading`, matches `search`.
///
/// Case-insensitive and trimmed on both sides; a row matches when its name or
/// its repository heading contains the search text. An empty search matches
/// every row.
pub(crate) fn matches(search: &str, heading: &str, entry: &ChangeEntry) -> bool {
    let needle = search.trim().to_lowercase();
    needle.is_empty()
    || entry.name.to_lowercase().contains(&needle)
    || heading.to_lowercase().contains(&needle)
}

/// Clears `filter` when it names a repository heading no longer among
/// `headings` - e.g. the workspace's last agent in that repository left.
pub(crate) fn reset_if_missing(filter: &mut Option<String>, headings: &[String]) {
    if let Some(heading) = filter
       && !headings.iter().any(|candidate| candidate == heading)
    {
        *filter = None;
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn entry(name: &str) -> ChangeEntry {
        ChangeEntry { name: name.to_string(),
                      dir:  PathBuf::from(name),
                      why:  None, }
    }

    /// Spec scenario: "Searching by name" - `LOGIN` shows only `add-login`
    /// of `add-login`/`add-export`.
    #[test]
    fn search_matches_name_case_insensitively() {
        assert!(matches("LOGIN", "acme/widget", &entry("add-login")));
        assert!(!matches("LOGIN", "acme/widget", &entry("add-export")));
    }

    #[test]
    fn search_matches_repository_heading() {
        assert!(matches("acme", "acme/widget", &entry("add-login")));
        assert!(!matches("acme", "other/widget", &entry("add-login")));
    }

    #[test]
    fn empty_search_matches_every_row() {
        assert!(matches("", "acme/widget", &entry("add-login")));
        assert!(matches("   ", "acme/widget", &entry("add-login")));
    }

    #[test]
    fn search_ignores_surrounding_whitespace() {
        assert!(matches("  login  ", "acme/widget", &entry("add-login")));
    }

    #[test]
    fn reset_if_missing_clears_a_filter_naming_an_absent_repository() {
        let mut filter = Some("acme/widget".to_string());
        reset_if_missing(&mut filter, &["other/repo".to_string()]);
        assert_eq!(filter, None);
    }

    #[test]
    fn reset_if_missing_keeps_a_filter_naming_a_present_repository() {
        let mut filter = Some("acme/widget".to_string());
        reset_if_missing(&mut filter,
                         &["acme/widget".to_string(), "other/repo".to_string()]);
        assert_eq!(filter, Some("acme/widget".to_string()));
    }

    #[test]
    fn reset_if_missing_leaves_no_filter_alone() {
        let mut filter = None;
        reset_if_missing(&mut filter, &["acme/widget".to_string()]);
        assert_eq!(filter, None);
    }
}
