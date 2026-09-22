//! Unit tests for [`super`]: the entry sources, the registry that joins
//! them, and slash-token detection.

use std::fs;
use std::path::PathBuf;

use super::*;
use crate::panel_commands::builtin;
use crate::panel_commands::skills;

/// Writes `<root>/<name>/SKILL.md` with `body` as its contents.
fn write_skill(root: &std::path::Path, name: &str, body: &str) {
    let dir = root.join(name);
    fs::create_dir_all(&dir).expect("create skill dir");
    fs::write(dir.join("SKILL.md"), body).expect("write SKILL.md");
}

fn skill_file(name: &str, description: &str) -> String {
    format!("---\nname: {name}\ndescription: {description}\n---\n\n# {name}\n")
}

#[test]
fn builtin_commands_are_complete_entries() {
    for entry in builtin::BuiltinCommands.entries() {
        assert!(!entry.token.contains('/'),
                "{} carries a slash; the lookup adds it on insertion",
                entry.token);
        assert!(!entry.token.is_empty(),
                "a built-in command has an empty token");
        assert!(!entry.description.is_empty(),
                "{} has no description",
                entry.token);
    }
}

#[test]
fn builtin_description_keys_resolve() {
    for key in builtin::command_keys() {
        assert_ne!(knot_core::l10n::t(key),
                   key,
                   "{key} should resolve to its localized copy");
    }
}

#[test]
fn a_root_with_two_skills_yields_two_entries() {
    let root = tempfile::tempdir().expect("tempdir");
    write_skill(root.path(),
                "alpha",
                &skill_file("alpha", "The first skill"));
    write_skill(root.path(), "beta", &skill_file("beta", "The second skill"));

    let entries = skills::SkillRoots::from_roots(vec![root.path().to_path_buf()]).entries();

    assert_eq!(entries,
               vec![LookupEntry::new("alpha", "The first skill"),
                    LookupEntry::new("beta", "The second skill")]);
}

#[test]
fn a_missing_root_yields_no_entries_without_error() {
    let missing = PathBuf::from("/knot/does/not/exist/skills");

    let entries = skills::SkillRoots::from_roots(vec![missing]).entries();

    assert!(entries.is_empty(), "a missing root should read as empty");
}

#[test]
fn a_skill_without_frontmatter_is_skipped() {
    let root = tempfile::tempdir().expect("tempdir");
    write_skill(root.path(), "bare", "# Just a heading\n");
    write_skill(root.path(),
                "named",
                &skill_file("named", "Has frontmatter"));

    let entries = skills::SkillRoots::from_roots(vec![root.path().to_path_buf()]).entries();

    assert_eq!(entries, vec![LookupEntry::new("named", "Has frontmatter")]);
}

#[test]
fn a_block_scalar_description_folds_onto_one_line() {
    let root = tempfile::tempdir().expect("tempdir");
    write_skill(root.path(),
                "folded",
                "---\nname: folded\ndescription: >-\n  first line\n  second line\n---\n");

    let entries = skills::SkillRoots::from_roots(vec![root.path().to_path_buf()]).entries();

    assert_eq!(entries,
               vec![LookupEntry::new("folded", "first line second line")]);
}

#[test]
fn both_sources_feed_one_registry_and_filter_by_prefix() {
    let root = tempfile::tempdir().expect("tempdir");
    write_skill(root.path(),
                "sendoff",
                &skill_file("sendoff", "A parting skill"));
    let builtin = builtin::BuiltinCommands;
    let skills = skills::SkillRoots::from_roots(vec![root.path().to_path_buf()]);

    let registry = LookupRegistry::from_sources(&[&builtin, &skills]);
    let tokens: Vec<&str> = registry.matching("send")
                                    .into_iter()
                                    .map(|entry| entry.token.as_str())
                                    .collect();

    assert!(tokens.contains(&"send"),
            "the built-in command should match");
    assert!(tokens.contains(&"sendoff"), "the skill should match");
}

#[test]
fn an_earlier_source_keeps_a_token_a_later_one_repeats() {
    let root = tempfile::tempdir().expect("tempdir");
    write_skill(root.path(),
                "send",
                &skill_file("send", "A skill shadowing a command"));
    let builtin = builtin::BuiltinCommands;
    let skills = skills::SkillRoots::from_roots(vec![root.path().to_path_buf()]);

    let registry = LookupRegistry::from_sources(&[&builtin, &skills]);
    let sends: Vec<&LookupEntry> = registry.matching("")
                                           .into_iter()
                                           .filter(|entry| entry.token == "send")
                                           .collect();

    assert_eq!(sends.len(), 1, "a skill must not shadow a built-in command");
    assert_ne!(sends[0].description, "A skill shadowing a command");
}

#[test]
fn an_empty_filter_matches_every_entry() {
    let registry = LookupRegistry::from_sources(&[&builtin::BuiltinCommands]);

    assert_eq!(registry.matching("").len(),
               builtin::BuiltinCommands.entries().len());
}

#[test]
fn a_filter_matching_nothing_selects_nothing() {
    let registry = LookupRegistry::from_sources(&[&builtin::BuiltinCommands]);

    assert!(registry.matching("zzzz-no-such-entry").is_empty());
}

#[test]
fn the_description_matches_as_well_as_the_token() {
    let root = tempfile::tempdir().expect("tempdir");
    write_skill(root.path(),
                "alpha",
                &skill_file("alpha", "Reticulates splines"));
    let skills = skills::SkillRoots::from_roots(vec![root.path().to_path_buf()]);

    let registry = LookupRegistry::from_sources(&[&skills]);

    assert_eq!(registry.matching("splines").len(), 1);
}

#[test]
fn a_lone_slash_is_an_active_token_with_an_empty_filter() {
    let token = active_token("/", 1).expect("a lone slash opens the lookup");

    assert_eq!(token.range, 0..1);
    assert_eq!(token.filter, "");
}

#[test]
fn typing_after_the_slash_fills_the_filter() {
    let token = active_token("/p", 2).expect("a partial token stays active");

    assert_eq!(token.range, 0..2);
    assert_eq!(token.filter, "p");
}

#[test]
fn a_slash_mid_prose_is_not_a_token() {
    assert_eq!(active_token("see the /docs", 13), None);
}

#[test]
fn a_token_ends_at_whitespace() {
    let token = active_token("/p mid-edit", 2).expect("the caret is still in the token");

    assert_eq!(token.range, 0..2);
    assert_eq!(token.filter, "p");
}

#[test]
fn a_caret_past_the_token_closes_the_lookup() {
    assert_eq!(active_token("/p mid-edit", 7), None);
}

#[test]
fn a_token_on_a_later_line_is_active() {
    let value = "first line\n/se";

    let token = active_token(value, value.len()).expect("the caret's line starts with a slash");

    assert_eq!(token.range, 11..14);
    assert_eq!(token.filter, "se");
}

#[test]
fn leading_whitespace_still_leaves_the_slash_first() {
    let token = active_token("  /se", 5).expect("indent does not hide the token");

    assert_eq!(token.range, 2..5);
    assert_eq!(token.filter, "se");
}

#[test]
fn a_line_without_a_slash_has_no_token() {
    assert_eq!(active_token("plain text", 4), None);
}
