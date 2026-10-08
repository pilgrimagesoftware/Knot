//! Unit tests for [`super`].

use super::{RepoSlug, github_repo};

fn slug(owner: &str, repo: &str) -> RepoSlug {
    RepoSlug { owner: owner.to_owned(),
               repo:  repo.to_owned(), }
}

#[test]
fn https_without_dot_git() {
    assert_eq!(github_repo("https://github.com/acme/widget"),
               Some(slug("acme", "widget")));
}

#[test]
fn https_with_dot_git() {
    assert_eq!(github_repo("https://github.com/acme/widget.git"),
               Some(slug("acme", "widget")));
}

#[test]
fn ssh_shorthand_without_dot_git() {
    assert_eq!(github_repo("git@github.com:acme/widget"),
               Some(slug("acme", "widget")));
}

#[test]
fn ssh_shorthand_with_dot_git() {
    assert_eq!(github_repo("git@github.com:acme/widget.git"),
               Some(slug("acme", "widget")));
}

#[test]
fn ssh_url_without_dot_git() {
    assert_eq!(github_repo("ssh://git@github.com/acme/widget"),
               Some(slug("acme", "widget")));
}

#[test]
fn ssh_url_with_dot_git() {
    assert_eq!(github_repo("ssh://git@github.com/acme/widget.git"),
               Some(slug("acme", "widget")));
}

#[test]
fn a_non_github_host_is_none() {
    assert_eq!(github_repo("https://gitlab.com/acme/widget"), None);
    assert_eq!(github_repo("git@gitlab.com:acme/widget.git"), None);
}

#[test]
fn malformed_input_is_none() {
    assert_eq!(github_repo("not a url"), None);
    assert_eq!(github_repo(""), None);
    assert_eq!(github_repo("https://github.com/acme"), None);
    assert_eq!(github_repo("https://github.com/"), None);
    assert_eq!(github_repo("https://github.com/acme/widget/extra"), None);
}

#[test]
fn display_is_owner_slash_repo() {
    assert_eq!(slug("acme", "widget").to_string(), "acme/widget");
}
