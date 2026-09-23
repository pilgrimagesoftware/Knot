//! Unit tests for [`super`]: the entry sources, the registry that joins
//! them, and which token the caret is in.

use std::fs;
use std::path::PathBuf;

use super::*;
use crate::panel_commands::builtin;
use crate::panel_commands::entry::Matcher;
use crate::panel_commands::skills;
use crate::panel_commands::token::Trigger;

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
    let tokens: Vec<String> = registry.matching("send")
                                      .into_iter()
                                      .map(|found| found.entry.token)
                                      .collect();

    assert!(tokens.iter().any(|token| token == "send"),
            "the built-in command should match");
    assert!(tokens.iter().any(|token| token == "sendoff"),
            "the skill should match");
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
    let sends: Vec<LookupEntry> = registry.matching("")
                                          .into_iter()
                                          .map(|found| found.entry)
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

#[test]
fn a_slash_token_reports_the_slash_trigger() {
    let token = active_token("/se", 3).expect("a slash token");

    assert_eq!(token.trigger, Trigger::Slash);
    assert_eq!(token.trigger.char(), '/');
}

#[test]
fn a_mention_after_a_space_is_an_active_token() {
    let token = active_token("look at @lib", 12).expect("an `@` after a space opens the lookup");

    assert_eq!(token.trigger, Trigger::Mention);
    assert_eq!(token.range, 8..12);
    assert_eq!(token.filter, "lib");
}

#[test]
fn a_bare_at_is_an_active_token_with_an_empty_filter() {
    let token = active_token("@", 1).expect("the trigger alone opens the lookup");

    assert_eq!(token.trigger, Trigger::Mention);
    assert_eq!(token.filter, "");
}

/// `panel-file-mentions`: "`@` mid-word is not a trigger".
#[test]
fn an_at_inside_an_email_address_is_not_a_token() {
    assert_eq!(active_token("write to paul@example.com", 25),
               None,
               "the `@` follows a letter, so the lookup must stay shut");
}

/// Task 6.1's own check: the two lookups are mutually exclusive because
/// the caret decides, not because two flags are kept in step.
#[test]
fn a_buffer_holding_both_reports_the_one_the_caret_is_in() {
    let value = "/review @lib.rs";
    let at_slash = active_token(value, 4).expect("the caret is in the slash token");
    let at_mention = active_token(value, 12).expect("the caret is in the mention");

    assert_eq!(at_slash.trigger, Trigger::Slash);
    assert_eq!(at_slash.filter, "review");
    assert_eq!(at_mention.trigger, Trigger::Mention);
    // The filter is the whole token, not the text up to the caret - the
    // same rule the slash lookup has always used, so a caret in the middle
    // of a token still filters on all of it.
    assert_eq!(at_mention.filter, "lib.rs");
    assert_ne!(at_slash.trigger, at_mention.trigger,
               "one caret position cannot be in both, which is what makes the popup single");
}

/// Moving the caret between them switches which list the popup shows -
/// the scenario `panel-slash-commands` adds for the slash lookup yielding.
#[test]
fn moving_the_caret_between_tokens_switches_the_trigger() {
    let value = "/review the diff @src/lib.rs";
    let triggers: Vec<Option<Trigger>> =
        [1, 7, 12, 18, 27].iter()
                          .map(|caret| active_token(value, *caret).map(|t| t.trigger))
                          .collect();

    assert_eq!(triggers,
               vec![Some(Trigger::Slash),
                    Some(Trigger::Slash),
                    None,
                    Some(Trigger::Mention),
                    Some(Trigger::Mention)],
               "the caret in the prose between them opens neither");
}

/// A path typed as a mention keeps its slashes: the mention claimed the
/// run, and a slash only triggers at the head of a line anyway.
#[test]
fn a_mention_holding_slashes_stays_one_mention() {
    let token = active_token("@crates/knot/src", 16).expect("a mention");

    assert_eq!(token.trigger, Trigger::Mention);
    assert_eq!(token.filter, "crates/knot/src");
}

/// Entries standing in for a repository's files, path as token.
fn paths(paths: &[&str]) -> Vec<LookupEntry> {
    paths.iter()
         .map(|path| LookupEntry::new(*path, ""))
         .collect()
}

/// The tokens `filter` selects from `entries`, best first.
fn ranked(entries: &[LookupEntry], filter: &str) -> Vec<String> {
    Matcher::Subsequence.matching(entries, filter)
                        .into_iter()
                        .map(|found| found.entry.token)
                        .collect()
}

/// `panel-file-mentions`' own example: "A subsequence matches".
#[test]
fn a_subsequence_reaches_a_path_no_substring_would() {
    let entries = paths(&["crates/knot-git/src/lib.rs",
                          "crates/knot/src/main.rs",
                          "README.md"]);

    let found = ranked(&entries, "kgs");

    assert!(found.contains(&"crates/knot-git/src/lib.rs".to_string()),
            "`kgs` should reach it as k-g-s across the path, which no substring search finds");
}

/// `panel-file-mentions`: "Name matches outrank directory matches".
#[test]
fn a_name_match_outranks_a_directory_match() {
    let entries = paths(&["lib/helper/main.rs", "crates/knot/src/lib.rs"]);

    let found = ranked(&entries, "lib");

    assert_eq!(found.first().map(String::as_str),
               Some("crates/knot/src/lib.rs"),
               "the one whose file name is `lib.rs` should come first, not the one under a \
                directory called `lib`");
}

/// `panel-file-mentions`: consecutive matched characters outrank
/// scattered ones.
#[test]
fn consecutive_characters_outrank_scattered_ones() {
    let entries = paths(&["a/b/c/refactor.rs", "src/rust_examples/file_actions.rs"]);

    let found = ranked(&entries, "rfa");

    assert_eq!(found.first().map(String::as_str),
               Some("a/b/c/refactor.rs"),
               "`rfa` sits inside `refactor` as a near-run, which should beat three characters \
                scattered across three segments");
}

/// A filter that is not a subsequence selects nothing - there is no typo
/// tolerance, which is what keeps a list of thousands honest.
#[test]
fn a_filter_that_is_not_a_subsequence_matches_nothing() {
    let entries = paths(&["crates/knot/src/lib.rs"]);

    assert!(ranked(&entries, "zzz").is_empty());
    assert!(ranked(&entries, "srcx").is_empty(),
            "one missing character is a miss, not a fuzzy near-match");
}

#[test]
fn matching_ignores_case_in_both_directions() {
    let entries = paths(&["Crates/Knot/README.md"]);

    assert_eq!(ranked(&entries, "readme").len(), 1);
    assert_eq!(ranked(&entries, "CRATES").len(), 1);
}

/// An empty filter lists everything, so opening the lookup on a bare `@`
/// shows the folder rather than nothing.
#[test]
fn an_empty_filter_lists_every_path() {
    let entries = paths(&["a.rs", "b.rs", "c.rs"]);

    assert_eq!(ranked(&entries, "").len(), 3);
}

/// `panel-file-mentions`: "The matched characters SHALL be marked in each
/// listed row, so the user can see why a row is a match."
#[test]
fn a_match_reports_where_it_matched() {
    let entries = paths(&["src/lib.rs"]);

    let found = Matcher::Subsequence.matching(&entries, "sl");
    let matched = &found.first().expect("a match").matched;

    assert_eq!(matched.len(), 2, "one offset per filter character");
    let marked: String = matched.iter()
                                .map(|offset| entries[0].token[*offset..].chars().next().unwrap())
                                .collect();
    assert_eq!(marked.to_lowercase(),
               "sl",
               "the marked offsets have to point at the characters that actually matched");
}

/// Substring matching keeps registry order and marks nothing - there is
/// nothing surprising to explain about a contiguous hit, and the declared
/// order of a few dozen commands carries meaning a score would destroy.
#[test]
fn substring_matching_keeps_its_order_and_marks_nothing() {
    let entries = paths(&["zeta", "alpha", "zebra"]);

    let found = Matcher::Substring.matching(&entries, "z");

    assert_eq!(found.iter()
                    .map(|f| f.entry.token.as_str())
                    .collect::<Vec<_>>(),
               vec!["zeta", "zebra"],
               "declared order, not alphabetical and not scored");
    assert!(found.iter().all(|f| f.matched.is_empty()));
}
