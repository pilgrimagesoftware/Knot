//! The git panel's window side: the reads that keep git off the render path,
//! the element tree, the staging actions and the working-tree watch.
//!
//! Split from `crate::git_panel`, which holds the parts that need no window -
//! the cache types, the section grouping and the relevance predicate. What
//! lives here needs `WorkspaceWindow`'s `pub(super)` fields, so it has to sit
//! inside `workspace_window` rather than beside the pure code it uses.
//!
//! Contract: `openspec/specs/git-panel-ui/spec.md`.

pub(super) mod actions;
mod reads;
mod view;
mod watch;
