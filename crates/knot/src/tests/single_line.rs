//! `single_line`: what a render gets when it declares one line and is handed
//! text it does not control - an agent's tool-call title, a user's queued
//! prompt, a status set over MCP, a path.

use crate::app_support::single_line;

#[test]
fn a_single_line_string_passes_through_unchanged() {
    assert_eq!(single_line("cargo test --workspace"),
               "cargo test --workspace");
}

#[test]
fn line_breaks_become_single_spaces() {
    assert_eq!(single_line("git commit -m 'one'\ngit push"),
               "git commit -m 'one' git push");
}

/// The case that produced the defect: a heredoc arrives indented, and
/// replacing only `\n` would leave that indentation in the middle of the
/// collapsed header.
#[test]
fn indentation_and_tabs_collapse_rather_than_surviving() {
    assert_eq!(single_line("cat <<'EOF'\n        line one\n\tline two\nEOF"),
               "cat <<'EOF' line one line two EOF");
}

#[test]
fn runs_of_spaces_collapse() {
    assert_eq!(single_line("grep     -rn  needle"), "grep -rn needle");
}

#[test]
fn leading_and_trailing_whitespace_go() {
    assert_eq!(single_line("  \n  make lint  \n "), "make lint");
}

#[test]
fn an_empty_string_stays_empty() {
    assert_eq!(single_line(""), "");
}

#[test]
fn a_whitespace_only_string_comes_back_empty() {
    assert_eq!(single_line(" \n\t  \n "), "");
}
