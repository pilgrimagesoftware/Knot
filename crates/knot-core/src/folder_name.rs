//! The name a folder gives the agent that works in it.
//!
//! Contract: `openspec/specs/agent-editor-ui/spec.md` - "Choosing a folder
//! names an unnamed agent after it".
//!
//! One rule with two readers: the agent editor fills its blank name field
//! from it when the user picks a folder, and `AgentStore::create` falls back
//! to it for a caller that supplies no name at all. They must agree - an
//! agent named one thing by the dialog and another by the store is the same
//! defect either way round - so the rule lives here rather than once in each
//! crate.

use std::path::Path;

/// The last component of `folder`, or `None` when it has none.
///
/// `None` rather than a fallback, because the two callers want different
/// things from that case: the editor leaves its name field alone, so the user
/// sees the blank field they already had, while the store has no field to
/// leave alone and substitutes the path itself. Returning a `String` here
/// would force one of those choices on both.
///
/// A root (`/`), an empty string and a path ending in `..` all have no last
/// component.
pub fn folder_name(folder: &str) -> Option<String> {
    Path::new(folder).file_name()
                     .map(|name| name.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests;
