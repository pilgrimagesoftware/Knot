//! Slash tokens and `@` mentions: what counts as one, and what is prose
//! that merely contains the character.

use crate::composer_scan::Construct;
use crate::composer_scan::escape_token;
use crate::composer_scan::scan;
use crate::composer_scan::tests::found;
use crate::composer_scan::tests::slices;

#[test]
fn a_leading_slash_is_a_token() {
    assert_eq!(found("/review the diff", Construct::SlashToken),
               vec!["/review"]);
}

/// `panel-rich-input`: "A token being typed is styled as it grows" - the
/// treatment starts at the trigger, before there is anything to resolve.
#[test]
fn a_bare_slash_is_already_a_token() {
    assert_eq!(found("/", Construct::SlashToken), vec!["/"]);
}

/// The composer does not ask the registry whether a command exists;
/// whether a token resolves is the lookup's business.
#[test]
fn an_unresolvable_token_is_still_a_token() {
    assert_eq!(found("/nosuchcommand", Construct::SlashToken),
               vec!["/nosuchcommand"]);
}

/// `panel-rich-input`: "A slash inside a path is prose".
#[test]
fn a_path_is_not_a_token() {
    assert!(found("crates/knot/src", Construct::SlashToken).is_empty(),
            "a slash mid-word is a separator, and a path full of them is still one thing");
}

/// The rule is the one `panel_commands::active_token` already applies:
/// first non-whitespace character of the line.
#[test]
fn a_slash_is_a_token_only_at_the_head_of_its_line() {
    assert!(found("please /review", Construct::SlashToken).is_empty());
    assert_eq!(found("first\n/review", Construct::SlashToken),
               vec!["/review"]);
    assert_eq!(found("   /review", Construct::SlashToken),
               vec!["/review"],
               "indentation before the slash still leaves it leading");
}

#[test]
fn a_mention_after_a_space_is_a_token() {
    assert_eq!(found("look at @crates/knot-git/src/lib.rs now",
                     Construct::Mention),
               vec!["@crates/knot-git/src/lib.rs"]);
}

#[test]
fn a_mention_at_the_start_of_the_buffer_is_a_token() {
    assert_eq!(found("@README.md", Construct::Mention), vec!["@README.md"]);
}

/// `panel-rich-input`: "An email address is prose".
#[test]
fn an_email_address_is_not_a_mention() {
    assert!(found("write to paul@example.com", Construct::Mention).is_empty(),
            "the `@` follows a letter, so it is part of an address rather than a trigger");
}

/// A path is one token, so a mention and the slashes inside it do not
/// fight: the slash rule needs the head of a line, and the mention has
/// already claimed the run.
#[test]
fn a_mention_holding_slashes_is_one_token() {
    let text = "@crates/knot/src";
    assert_eq!(found(text, Construct::Mention), vec!["@crates/knot/src"]);
    assert!(found(text, Construct::SlashToken).is_empty());
}

#[test]
fn several_mentions_on_one_line_are_separate_tokens() {
    assert_eq!(found("diff @a.rs against @b.rs", Construct::Mention),
               vec!["@a.rs", "@b.rs"]);
}

/// A token and a slash command can both be live in one buffer; which one
/// the *lookup* answers to is the caret's business, not the styling's.
#[test]
fn a_buffer_can_hold_both_kinds_of_token() {
    let text = "/review\nand @lib.rs";
    assert_eq!(found(text, Construct::SlashToken), vec!["/review"]);
    assert_eq!(found(text, Construct::Mention), vec!["@lib.rs"]);
}

/// An attachment's reference is its own construct, so the chip treatment
/// can sit above the token one without the two being the same range.
#[test]
fn an_attachment_reference_is_found_wherever_it_sits() {
    let text = "see @shot.png and @shot.png again";
    let spans = scan(text, &["@shot.png"]);
    assert_eq!(slices(text, &spans, Construct::Attachment),
               vec!["@shot.png", "@shot.png"],
               "the same file attached twice is two chips, not one");
}

/// A path with a space stays one token, because the space is escaped.
/// Without this the composer styles `@my` and the lookup completes over a
/// fragment, which is the same defect seen from two directions.
#[test]
fn an_escaped_space_does_not_end_a_token() {
    assert_eq!(found(r"@my\ notes/today.md here", Construct::Mention),
               vec![r"@my\ notes/today.md"],
               "the escaped space is inside the token; the unescaped one ends it");
}

#[test]
fn an_unescaped_space_still_ends_a_token() {
    assert_eq!(found("@notes.md and more", Construct::Mention),
               vec!["@notes.md"]);
}

/// Escaping and scanning are one rule read in two directions, so what the
/// insertion writes has to be what the scanner reads back as a token.
#[test]
fn what_escaping_writes_the_scanner_reads_as_one_token() {
    for path in ["plain.rs",
                 "my notes/today.md",
                 "two  spaces.md",
                 r"back\slash.rs",
                 "trailing space .md"]
    {
        let buffer = format!("see @{} end", escape_token(path));
        let found = found(&buffer, Construct::Mention);

        assert_eq!(found.len(), 1, "{path:?} produced {found:?}");
        assert_eq!(found[0],
                   format!("@{}", escape_token(path)),
                   "the whole escaped path has to be the token");
    }
}

#[test]
fn escaping_leaves_an_ordinary_path_alone() {
    assert_eq!(escape_token("crates/knot-git/src/lib.rs"),
               "crates/knot-git/src/lib.rs",
               "a path with nothing to protect must not grow backslashes");
}
