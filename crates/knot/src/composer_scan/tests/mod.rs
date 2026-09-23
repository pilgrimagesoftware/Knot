//! The scanner's tests, by what they are about.
//!
//! None of these opens a window. That is the point of the scanner being a
//! function over `&str`: the composer's hardest logic is the part that
//! needs no GPUI at all, so it is checked where checking is cheap.
//!
//! They are not coverage of anything on screen. Nothing draws these spans
//! yet - the module is `UNWIRED(#396)` until the decoration collections
//! land - so a green run here says the ranges are right, not that the user
//! can see them.

mod incremental;
mod markdown;
mod tokens;

use crate::composer_scan::Construct;
use crate::composer_scan::Span;
use crate::composer_scan::scan;

/// The slices of `text` that `scan` classified as `construct`, in order.
///
/// Asserting on the text rather than on offsets is deliberate: a test that
/// says `["**ship it**"]` survives an edit to the fixture above it, and a
/// test that says `[12..23]` does not.
fn found(text: &str, construct: Construct) -> Vec<&str> {
    slices(text, &scan(text, &[]), construct)
}

/// As [`found`], over spans already in hand.
fn slices<'a>(text: &'a str, spans: &[Span], construct: Construct) -> Vec<&'a str> {
    spans.iter()
         .filter(|span| span.construct == construct)
         .map(|span| &text[span.range.clone()])
         .collect()
}
