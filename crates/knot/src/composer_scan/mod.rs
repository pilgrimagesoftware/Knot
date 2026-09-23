//! What the panel composer's buffer is made of, as byte ranges.
//!
//! The composer draws slash tokens, `@` mentions, markdown and attachment
//! references differently from prose (`openspec/specs/panel-rich-input`).
//! Deciding *where* those runs are is this module's whole job; turning a
//! run into a colour is the decoration layer's, and nothing here knows
//! about GPUI, a theme, or a window. That is what lets the whole of it be
//! tested without opening one.
//!
//! - [`scan`] classifies a buffer from scratch.
//! - [`rescan`] does the same for an edit, touching only what the edit can have
//!   changed. The composer re-renders on every keystroke, so a full re-parse
//!   per character is a defect rather than a tuning question
//!   (`.claude/rules/rust-structure.md`, "No I/O on the render path" - the same
//!   argument, one step further in).
//!
//! The two must agree. [`rescan`] is only ever an optimisation of
//! [`scan`], and `tests/incremental.rs` is what holds it to that.

mod dirty;
mod markdown;
mod scan;
mod tokens;

#[cfg(test)]
mod tests;

pub(crate) use scan::Construct;
pub(crate) use scan::Edit;
pub(crate) use scan::Span;
pub(crate) use scan::rescan;
pub(crate) use scan::scan;
