//! The Markdown `TextView` both Markdown surfaces are built from: the ACP
//! panel's assistant messages and the pane `display-markdown` opens.
//!
//! Contract: `openspec/specs/acp-panel-ui/spec.md` - rendered Markdown draws
//! body text in the UI font and headers in the title font.
//!
//! One constructor, [`markdown_view`], so the two surfaces cannot drift: the
//! spec requires them to render alike, and a shared constructor makes that
//! true by construction rather than by review.
//!
//! # Why a heading renderer exists here at all
//!
//! gpui-kit exposes no per-heading font family. `TextViewStyle` carries
//! heading *sizes* (`heading_base_font_size`, `with_heading_font_size`) and
//! `StyleRefinement`s for code blocks and tables, but nothing for a heading's
//! face, and `gpui-base` renders a heading as a plain `div` that inherits the
//! ambient family. Giving headers their own face therefore means claiming the
//! heading node through the block parser and renderer hooks and drawing it
//! here.
//!
//! # The upgrade check this implies
//!
//! Claiming the node means `gpui-base`'s own heading branch no longer runs, so
//! [`HEADING_SIZE_FACTORS`], [`HEADING_WEIGHTS`] and
//! [`HEADING_BOTTOM_PADDING_REMS`] duplicate its values - as of gpui-kit
//! 0.6.4, `gpui-base`'s `BlockNode::render_block`. A gpui-kit upgrade that
//! changes a heading's size, weight or padding will leave these disagreeing
//! with the built-in rendering, silently. Check them against that function
//! when bumping gpui-kit.
//!
//! A heading-face refinement upstream would delete this whole module; until
//! then the duplication is the cost of the second face.

use gpui_kit::component::text::{MarkdownNode, MarkdownParseContext, TextView, markdown_ast};
use gpui_kit::{
    App, ElementId, FontWeight, ParentElement, Pixels, SharedString, Styled, Window, div, rems,
};

/// The name the heading block parser claims and the block renderer answers to.
/// Namespaced so it cannot collide with a built-in node name.
const HEADING_NODE_NAME: &str = "knot-heading";

/// Size factor per heading level, indexed by `level - 1`, multiplied by the
/// body text size. `gpui-base`'s own values - see the module doc's upgrade
/// check.
const HEADING_SIZE_FACTORS: [f32; 6] = [2.0, 1.5, 1.25, 1.125, 1.0, 1.0];

/// Font weight per heading level, indexed by `level - 1`. `gpui-base`'s own
/// values - see the module doc's upgrade check.
const HEADING_WEIGHTS: [FontWeight; 6] = [FontWeight::BOLD,
                                          FontWeight::SEMIBOLD,
                                          FontWeight::SEMIBOLD,
                                          FontWeight::SEMIBOLD,
                                          FontWeight::SEMIBOLD,
                                          FontWeight::MEDIUM];

/// A heading's bottom padding, in rems. `gpui-base` gives a heading this and
/// no other spacing, so matching it leaves the face as the only difference.
const HEADING_BOTTOM_PADDING_REMS: f32 = 0.3;

/// What a level outside 1-6 renders at. `gpui-base` falls through to body size
/// at normal weight rather than clamping to level 6, and a node carrying no
/// level payload is the same kind of "not a heading we know" case.
const HEADING_FALLBACK: (f32, FontWeight) = (1.0, FontWeight::NORMAL);

/// The payload a [`HEADING_NODE_NAME`] node carries. The heading's text
/// travels in the node's own text slot; only the level needs a payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HeadingLevel(pub(crate) u8);

/// The pixel size and weight a heading level draws at, against `body_size` as
/// the rem base.
///
/// `body_size` rather than `TextViewStyle::heading_base_font_size`, whose
/// default is a fixed 14px that no Knot code sets: scaling from the
/// configured Markdown body size is what makes a user who enlarges that size
/// get proportionally larger headers. The two cannot disagree, because
/// claiming the heading node leaves this the only live path.
pub(crate) fn heading_size_and_weight(level: u8, body_size: Pixels) -> (Pixels, FontWeight) {
    let (factor, weight) = match level {
        1..=6 => {
            let index = usize::from(level - 1);
            (HEADING_SIZE_FACTORS[index], HEADING_WEIGHTS[index])
        }
        _ => HEADING_FALLBACK,
    };
    (rems(factor).to_pixels(body_size), weight)
}

/// Flattens a heading's inline children to plain text, appending to `out`.
///
/// A header's marks are deliberately lost: the block renderer receives the
/// heading's text, not its parsed inline marks, so `## A **bold** word`
/// renders as "A bold word" - the words themselves, never the Markdown source
/// that marked them. See `openspec/changes/swap-ui-title-font-roles/design.md`
/// for why that loss was accepted.
fn push_inline_text(node: &markdown_ast::Node, out: &mut String) {
    match node {
        markdown_ast::Node::Text(text) => out.push_str(&text.value),
        markdown_ast::Node::InlineCode(code) => out.push_str(&code.value),
        markdown_ast::Node::InlineMath(math) => out.push_str(&math.value),
        // Every other inline construct - bold, italics, links, strikethrough
        // - contributes only through its children.
        other => {
            if let Some(children) = other.children() {
                for child in children {
                    push_inline_text(child, out);
                }
            }
        }
    }
}

/// Claims a heading node and leaves every other node to the built-in
/// conversion, declining it by returning `None`.
///
/// Separate from [`parse_heading`], which is the hook's shape, so this can be
/// tested against a hand-built AST node: `MarkdownParseContext` has no public
/// constructor, and the parse needs nothing from it.
pub(crate) fn heading_node(node: &markdown_ast::Node) -> Option<MarkdownNode> {
    let markdown_ast::Node::Heading(heading) = node
    else {
        return None;
    };
    let mut text = String::new();
    for child in &heading.children {
        push_inline_text(child, &mut text);
    }
    Some(MarkdownNode::new(HEADING_NODE_NAME, HeadingLevel(heading.depth)).text(text))
}

/// The block parser hook. Runs ahead of the built-in AST conversion for every
/// block node, which is what lets [`heading_node`] take headings over.
fn parse_heading(node: &markdown_ast::Node, _context: &MarkdownParseContext<'_>)
                 -> Option<MarkdownNode> {
    heading_node(node)
}

/// The block renderer for a claimed heading: the title family, the level's
/// size and weight, and the bottom padding `gpui-base` gives a heading.
///
/// Returns a concrete `Div` rather than `impl IntoElement`: an opaque return
/// type would capture the borrow of `node`, and the renderer hook's element
/// must outlive the call.
fn render_heading(node: &MarkdownNode, title_font_family: SharedString, body_size: Pixels)
                  -> gpui_kit::Div {
    let level = node.data::<HeadingLevel>().map_or(0, |level| level.0);
    let (text_size, font_weight) = heading_size_and_weight(level, body_size);
    div().pb(rems(HEADING_BOTTOM_PADDING_REMS))
         .whitespace_normal()
         .font_family(title_font_family)
         .text_size(text_size)
         .font_weight(font_weight)
         .child(node.as_text().to_string())
}

/// The configured Markdown view: body text in `ui_font_family`, headers in
/// `title_font_family` at the size and weight their level already gave them.
///
/// The body family is set on the `TextView` itself, which sets the ambient
/// family for everything it draws, so every body construct - paragraphs,
/// lists, table cells, quotes, the text around inline code - inherits it.
/// Headings do not: the heading renderer sets the family explicitly, and a
/// child's own family wins over the ambient one. Inline code and code blocks
/// keep the monospace family, which `TextViewStyle` sets from the theme.
///
/// No `TextViewStyle` is passed. Without one, gpui-kit derives it from the
/// live theme, which is what keeps code blocks, tables and selection colors
/// tracking the system appearance; a `TextViewStyle::default()` would pin them
/// to the light palette. Its heading sizes would not be read anyway - see
/// [`heading_size_and_weight`].
pub(crate) fn markdown_view(id: impl Into<ElementId>, source: impl Into<SharedString>,
                            ui_font_family: SharedString, title_font_family: SharedString,
                            body_size: Pixels)
                            -> TextView {
    TextView::markdown(id, source).font_family(ui_font_family)
                                  .markdown_block_parser(parse_heading)
                                  .markdown_block_renderer(HEADING_NODE_NAME,
                                                           move |node: &MarkdownNode,
                                                                 _window: &mut Window,
                                                                 _cx: &mut App| {
                                                               render_heading(node,
                                                                        title_font_family.clone(),
                                                                        body_size)
                                                           })
}
