//! Unit tests for [`crate::markdown_view`].
//!
//! The heading table is pinned level by level because claiming the heading
//! node takes `gpui-base`'s own branch out of play: nothing but these
//! assertions would notice the two drifting apart after a gpui-kit bump.
//!
//! The renderer itself needs a `Window` and an `App`, so what is tested here
//! is the pure part - the level-to-size-and-weight mapping, and the parser
//! against AST nodes built by hand. Built by hand rather than parsed from
//! Markdown because the `markdown` crate reaches `knot` only as a gpui-kit
//! re-export, and driving it would test that crate's parser rather than this
//! module's handling of what it produces.

use gpui_kit::component::text::markdown_ast;
use gpui_kit::{FontWeight, Pixels, px};

use crate::markdown_view::{HeadingLevel, heading_node, heading_size_and_weight};

/// The body size the factors multiply. Deliberately not 14, the default of
/// `TextViewStyle::heading_base_font_size`, so these assertions would fail if
/// the base silently reverted to that.
const BODY: f32 = 16.;

fn size_of(level: u8) -> Pixels {
    heading_size_and_weight(level, px(BODY)).0
}

fn weight_of(level: u8) -> FontWeight {
    heading_size_and_weight(level, px(BODY)).1
}

#[test]
fn every_heading_level_keeps_its_own_size() {
    assert_eq!(size_of(1), px(32.));
    assert_eq!(size_of(2), px(24.));
    assert_eq!(size_of(3), px(20.));
    assert_eq!(size_of(4), px(18.));
    assert_eq!(size_of(5), px(16.));
    assert_eq!(size_of(6), px(16.));
}

#[test]
fn every_heading_level_keeps_its_own_weight() {
    assert_eq!(weight_of(1), FontWeight::BOLD);
    assert_eq!(weight_of(2), FontWeight::SEMIBOLD);
    assert_eq!(weight_of(3), FontWeight::SEMIBOLD);
    assert_eq!(weight_of(4), FontWeight::SEMIBOLD);
    assert_eq!(weight_of(5), FontWeight::SEMIBOLD);
    assert_eq!(weight_of(6), FontWeight::MEDIUM);
}

/// A level outside 1-6 cannot come from Markdown, but it can come from a node
/// carrying no level payload. Body size at normal weight, as `gpui-base` falls
/// through to - not a panic on the index, and not a clamp to level 6's medium
/// weight.
#[test]
fn an_out_of_range_level_falls_back_to_body_size_at_normal_weight() {
    for level in [0, 7, u8::MAX] {
        assert_eq!(size_of(level), px(BODY), "level {level} should not scale");
        assert_eq!(weight_of(level), FontWeight::NORMAL, "level {level}");
    }
}

#[test]
fn the_heading_size_scales_with_the_body_size() {
    assert_eq!(heading_size_and_weight(1, px(10.)).0, px(20.));
    assert_eq!(heading_size_and_weight(1, px(20.)).0, px(40.));
}

fn text(value: &str) -> markdown_ast::Node {
    markdown_ast::Node::Text(markdown_ast::Text { value:    value.to_string(),
                                                  position: None, })
}

fn heading(depth: u8, children: Vec<markdown_ast::Node>) -> markdown_ast::Node {
    markdown_ast::Node::Heading(markdown_ast::Heading { depth,
                                                        children,
                                                        position: None })
}

#[test]
fn a_heading_is_claimed_with_its_level_and_text() {
    let node = heading_node(&heading(2, vec![text("A heading")])).expect("heading claimed");
    assert_eq!(node.data::<HeadingLevel>(), Some(&HeadingLevel(2)));
    assert_eq!(node.as_text(), "A heading");
}

/// The accepted loss: a mark inside a header renders as the words, never as
/// the Markdown source that marked them.
#[test]
fn inline_marks_inside_a_heading_flatten_to_their_words() {
    let bold = markdown_ast::Node::Strong(markdown_ast::Strong { children: vec![text("bold")],
                                                                 position: None, });
    let node =
        heading_node(&heading(2, vec![text("A "), bold, text(" word")])).expect("heading claimed");
    assert_eq!(node.as_text(), "A bold word");
}

/// Italics, inline code and a link each contribute their words and nothing of
/// their markup - the link its text, not its URL.
#[test]
fn every_inline_construct_contributes_only_its_words() {
    let italic = markdown_ast::Node::Emphasis(markdown_ast::Emphasis { children: vec![text("Italic")],
                                                                       position: None, });
    let code = markdown_ast::Node::InlineCode(markdown_ast::InlineCode { value:
                                                                             "code".to_string(),
                                                                         position: None, });
    let link = markdown_ast::Node::Link(markdown_ast::Link { children: vec![text("link")],
                                                             position: None,
                                                             url:
                                                                 "http://example.test".to_string(),
                                                             title:    None, });
    let node = heading_node(&heading(1,
                                     vec![italic,
                                          text(", "),
                                          code,
                                          text(" and a "),
                                          link])).expect("heading claimed");
    assert_eq!(node.as_text(), "Italic, code and a link");
}

#[test]
fn a_block_that_is_not_a_heading_is_left_to_the_built_in_conversion() {
    let paragraph = markdown_ast::Node::Paragraph(markdown_ast::Paragraph { children: vec![text("A para.")],
                                                                            position: None, });
    assert!(heading_node(&paragraph).is_none());

    let quote = markdown_ast::Node::Blockquote(markdown_ast::Blockquote { children: vec![paragraph],
                                                                          position: None, });
    assert!(heading_node(&quote).is_none());

    let code = markdown_ast::Node::Code(markdown_ast::Code { value:    "code".to_string(),
                                                             position: None,
                                                             lang:     None,
                                                             meta:     None, });
    assert!(heading_node(&code).is_none());
}
