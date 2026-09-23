//! Markdown constructs, and the rule that an unclosed one is prose.

use crate::composer_scan::Construct;
use crate::composer_scan::tests::found;

#[test]
fn strong_emphasis_keeps_both_marker_pairs() {
    assert_eq!(found("**ship it**", Construct::Strong),
               vec!["**ship it**"],
               "the markers are part of the run: styling never hides a character");
}

#[test]
fn emphasis_is_distinct_from_strong() {
    assert_eq!(found("*soon*", Construct::Emphasis), vec!["*soon*"]);
    assert!(found("*soon*", Construct::Strong).is_empty());
    assert_eq!(found("**soon**", Construct::Strong), vec!["**soon**"]);
    assert!(found("**soon**", Construct::Emphasis).is_empty(),
            "a strong run is not also two emphasis runs");
}

#[test]
fn underscores_work_like_asterisks() {
    assert_eq!(found("_soon_", Construct::Emphasis), vec!["_soon_"]);
    assert_eq!(found("__soon__", Construct::Strong), vec!["__soon__"]);
}

/// `panel-rich-input`: "An unclosed marker does not bleed".
#[test]
fn a_lone_asterisk_is_prose() {
    assert!(found("a * b", Construct::Emphasis).is_empty(),
            "the asterisk is followed by a space, so it opens nothing");
    assert!(found("half *open", Construct::Emphasis).is_empty(),
            "an opener with no closer must not italicise the rest of the line");
}

#[test]
fn inline_code_is_its_own_construct() {
    assert_eq!(found("run `make lint` first", Construct::InlineCode),
               vec!["`make lint`"]);
}

/// Emphasis markers inside a code span are literal text.
#[test]
fn code_spans_swallow_their_contents() {
    let text = "`a * b * c`";
    assert_eq!(found(text, Construct::InlineCode), vec!["`a * b * c`"]);
    assert!(found(text, Construct::Emphasis).is_empty());
}

#[test]
fn an_unclosed_backtick_is_prose() {
    assert!(found("a ` b", Construct::InlineCode).is_empty());
}

#[test]
fn a_fenced_block_covers_its_fences_and_contents() {
    let text = "before\n```\ncode line\n```\nafter";
    assert_eq!(found(text, Construct::CodeFence),
               vec!["```\ncode line\n```"]);
}

/// `panel-rich-input`: "An unclosed fence styles only what follows it".
#[test]
fn an_unclosed_fence_styles_what_follows_and_nothing_before() {
    let text = "prose above\n```\nstill typing";
    let fenced = found(text, Construct::CodeFence);
    assert_eq!(fenced, vec!["```\nstill typing"]);
    assert!(!fenced[0].contains("prose above"),
            "the text before the fence is not code");
}

/// `panel-rich-input`: "An info string does not change the treatment".
#[test]
fn the_info_string_does_not_change_the_treatment() {
    let rust = found("```rust\nlet x = 1;\n```", Construct::CodeFence);
    let python = found("```python\nx = 1\n```", Construct::CodeFence);
    assert_eq!(rust.len(), 1);
    assert_eq!(python.len(), 1);
    assert!(rust[0].starts_with("```rust") && python[0].starts_with("```python"),
            "both are one code run - the language is part of the text, not of the styling");
}

#[test]
fn markdown_inside_a_fence_is_not_styled() {
    let text = "```\n**not strong**\n```";
    assert_eq!(found(text, Construct::CodeFence).len(), 1);
    assert!(found(text, Construct::Strong).is_empty(),
            "a fence is one code treatment; what it quotes is not read as markdown");
}

#[test]
fn a_tilde_fence_is_a_fence() {
    assert_eq!(found("~~~\ncode\n~~~", Construct::CodeFence),
               vec!["~~~\ncode\n~~~"]);
}

#[test]
fn headings_carry_their_markers() {
    assert_eq!(found("## Plan", Construct::Heading), vec!["## Plan"]);
    assert!(found("#hashtag", Construct::Heading).is_empty(),
            "a hash run has to be followed by a space to be a heading");
    assert!(found("####### too deep", Construct::Heading).is_empty());
}

#[test]
fn a_list_marker_is_the_marker_and_not_the_item() {
    assert_eq!(found("- first\n- second", Construct::ListMarker),
               vec!["- ", "- "]);
    assert_eq!(found("1. first", Construct::ListMarker), vec!["1. "]);
}

#[test]
fn a_block_quote_covers_its_line() {
    assert_eq!(found("> quoted\nplain", Construct::BlockQuote),
               vec!["> quoted"]);
}

#[test]
fn a_link_is_one_construct() {
    assert_eq!(found("see [the spec](docs/spec.md) first", Construct::Link),
               vec!["[the spec](docs/spec.md)"]);
}

#[test]
fn an_unclosed_link_is_prose() {
    assert!(found("see [the spec", Construct::Link).is_empty());
    assert!(found("see [the spec] alone", Construct::Link).is_empty(),
            "a bracketed phrase with no target is not a link");
}

/// A prompt is mostly prose, and prose must come back clean - a scanner
/// that finds constructs everywhere is as wrong as one that finds none.
#[test]
fn ordinary_prose_yields_nothing() {
    let text = "Please look at the failing test and tell me what you think is wrong.";
    assert!(crate::composer_scan::scan(text, &[]).is_empty(),
            "nothing here is a construct");
}
