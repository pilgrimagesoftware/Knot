//! The git panel: an agent's working tree, the diff for a selected file, and
//! the staging and commit actions over them.
//!
//! `state` holds what the panel knows and the two caches it knows it through;
//! `sections` turns a status into drawable rows and answers whether a
//! selection survived a refresh.
//!
//! Contract: `openspec/specs/git-panel-ui/spec.md`. Ported from the Swift
//! app's `Skwad/Views/Git/`, with the divergences recorded in
//! `openspec/changes/git-panel-port/proposal.md`.

pub(crate) mod sections;
pub(crate) mod state;

#[cfg(test)]
mod tests;
