//! Fixture-file tests for the Claude subagent reader.
//!
//! The fixtures are written into a `tempfile` directory rather than committed,
//! so each case states the exact file it is about next to the assertion. The
//! three malformed shapes are the ones `data-import` requires be *reported*
//! rather than guessed at, which is what most of this file is checking.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::{parse_definition, read_dir_into};
use crate::import::result::UnreadableReason;
use crate::import::subagents::SubagentScan;

/// A well-formed definition, shaped like the real ones in `~/.claude/agents`:
/// frontmatter keys Knot does not read, then the system prompt.
const WELL_FORMED: &str = "---\nname: architect-reviewer\ndescription: Reviews \
                           architecture.\ncolor: gray\nmodel: claude-sonnet-5\n---\n\nYou are an \
                           expert software architect focused on maintaining architectural \
                           integrity.\n";

fn scan_dir(dir: &Path) -> SubagentScan {
    let mut scan = SubagentScan::default();
    read_dir_into(dir, &mut scan);
    scan
}

fn write(dir: &TempDir, name: &str, contents: &str) {
    fs::write(dir.path().join(name), contents).expect("fixture written");
}

#[test]
fn well_formed_definition_yields_name_and_body() {
    let definition = parse_definition(WELL_FORMED, Path::new("architect-review.md"))
        .expect("well-formed definition parses");

    assert_eq!(definition.name, "architect-reviewer");
    assert!(definition.instructions
                      .starts_with("You are an expert software architect"));
}

/// The file name is `architect-review`, the definition is
/// `architect-reviewer`: the frontmatter wins, so a rename of either cannot
/// silently change what the persona is called.
#[test]
fn name_comes_from_frontmatter_not_the_file_name() {
    let definition =
        parse_definition(WELL_FORMED, Path::new("architect-review.md")).expect("parses");

    assert_ne!(definition.name, "architect-review");
    assert_eq!(definition.name, "architect-reviewer");
}

#[test]
fn keys_knot_has_no_equivalent_for_are_dropped() {
    let definition = parse_definition(WELL_FORMED, Path::new("a.md")).expect("parses");

    for dropped in ["description:",
                    "color:",
                    "model:",
                    "Reviews architecture",
                    "claude-sonnet-5"]
    {
        assert!(!definition.instructions.contains(dropped),
                "{dropped} should not have been folded into the instructions");
    }
}

#[test]
fn a_file_with_no_frontmatter_is_unreadable() {
    let reason = parse_definition("Just a prompt, no frontmatter.\n", Path::new("a.md"))
        .expect_err("should not parse");

    assert_eq!(reason, UnreadableReason::NoFrontmatter);
}

#[test]
fn an_unclosed_frontmatter_block_is_unreadable() {
    let reason = parse_definition("---\nname: stuck\n\nbody\n", Path::new("a.md"))
        .expect_err("should not parse");

    assert_eq!(reason, UnreadableReason::NoFrontmatter);
}

/// The contract is explicit that the file name is not a fallback: a persona
/// imported under a guessed name is worse than one reported as unreadable.
#[test]
fn a_file_with_no_name_key_is_not_guessed_at() {
    let reason = parse_definition("---\ndescription: no name here\n---\n\nbody\n",
                                  Path::new("guess-me.md")).expect_err("should not parse");

    assert_eq!(reason, UnreadableReason::NoName);
}

#[test]
fn a_file_with_an_empty_body_is_unreadable() {
    let reason = parse_definition("---\nname: hollow\n---\n\n   \n", Path::new("a.md"))
        .expect_err("should not parse");

    assert_eq!(reason, UnreadableReason::EmptyBody);
}

#[test]
fn a_quoted_name_reads_the_same_as_a_bare_one() {
    let quoted = parse_definition("---\nname: \"quoted-one\"\n---\nbody\n", Path::new("a.md"))
        .expect("parses");

    assert_eq!(quoted.name, "quoted-one");
}

/// A `name:` inside another key's value is indented, and a top-level key is
/// not - which is the whole reason the scanner anchors at column zero.
#[test]
fn an_indented_name_inside_another_value_is_not_mistaken_for_the_key() {
    let text = "---\ndescription: |\n  name: not-the-name\nname: the-name\n---\nbody\n";

    let definition = parse_definition(text, Path::new("a.md")).expect("parses");

    assert_eq!(definition.name, "the-name");
}

/// `data-import`: one malformed file must not abandon the others.
#[test]
fn one_malformed_file_among_many_does_not_stop_the_rest() {
    let dir = TempDir::new().expect("temp dir");
    write(&dir, "good-one.md", WELL_FORMED);
    write(&dir, "good-two.md", "---\nname: second\n---\nAlso fine.\n");
    write(&dir, "broken.md", "no frontmatter at all\n");

    let scan = scan_dir(dir.path());

    let names: Vec<&str> = scan.definitions.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, ["architect-reviewer", "second"]);
    assert_eq!(scan.unreadable.len(), 1);
    assert_eq!(scan.unreadable[0].name, "broken.md");
    assert_eq!(scan.unreadable[0].reason, UnreadableReason::NoFrontmatter);
}

#[test]
fn non_markdown_files_are_ignored_entirely() {
    let dir = TempDir::new().expect("temp dir");
    write(&dir, "notes.txt", "not a definition");
    write(&dir, "config.json", "{}");

    let scan = scan_dir(dir.path());

    assert!(scan.is_empty(), "only .md files are definitions");
}

/// A tool that is not installed has no directory, which is nothing to import
/// rather than an error.
#[test]
fn a_missing_directory_yields_an_empty_scan() {
    let dir = TempDir::new().expect("temp dir");

    let scan = scan_dir(&dir.path().join("does-not-exist"));

    assert!(scan.is_empty());
}

/// Filesystem order differs between machines; the offered list must not.
#[test]
fn definitions_are_listed_in_a_stable_order() {
    let dir = TempDir::new().expect("temp dir");
    write(&dir, "zeta.md", "---\nname: zeta\n---\nz\n");
    write(&dir, "alpha.md", "---\nname: alpha\n---\na\n");

    let names: Vec<String> = scan_dir(dir.path()).definitions
                                                 .into_iter()
                                                 .map(|d| d.name)
                                                 .collect();

    assert_eq!(names, ["alpha", "zeta"]);
}

/// Reading must leave the source exactly as it was.
#[test]
fn reading_does_not_modify_the_source_files() {
    let dir = TempDir::new().expect("temp dir");
    write(&dir, "one.md", WELL_FORMED);
    let path = dir.path().join("one.md");
    let before = fs::read(&path).expect("read before");

    let _ = scan_dir(dir.path());

    assert_eq!(fs::read(&path).expect("read after"), before);
}
