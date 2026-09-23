//! Turning the composer's scanned runs into something the eye can tell
//! apart.
//!
//! [`crate::composer_scan`] says *where* the constructs are; this says
//! what each one looks like, and owns the decoration collections that put
//! it on screen. The split is the one the design asks for: the scanner
//! knows no theme and no GPUI, and this knows no markdown.
//!
//! - [`treatment`] resolves one construct against the active theme.
//! - [`ComposerStyling`] holds an agent's three collections and keeps them in
//!   step with its buffer.
//!
//! Three collections rather than one, created in a fixed order, because
//! `gpui-base` layers them in creation order and lets the first win a
//! contested property. That ordering *is* the precedence - attachment
//! chips, then tokens, then markdown - and it is why a `/command` inside a
//! fenced block still reads as a token.

mod layers;
mod treatment;

#[cfg(test)]
mod tests;

pub(crate) use layers::ComposerStyling;
pub(crate) use treatment::Palette;
