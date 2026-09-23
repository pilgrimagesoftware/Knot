//! One-time migration from the single `settings.json`.
//!
//! Contract: `openspec/specs/settings-persistence/spec.md`.
//!
//! An installation written by an earlier build holds every scalar and every
//! collection in one document in the application-data directory. On load,
//! when that document is present, its collections are lifted out, the new
//! documents are written, and the legacy document is renamed to
//! `settings.json.migrated` so it is never read again.
//!
//! Its `savedWorkspaces` is itself in the combined shape - each record
//! carrying the eight UI fields alongside the four configured ones - so the
//! workspaces go through [`super::workspace_split`] rather than straight into
//! [`Workspace`], and the two migrations compose: an installation still on
//! the legacy document arrives at both new documents, not at a combined one.
//!
//! The rule for a collection that both documents carry is that the new
//! document wins. That is what makes the migration idempotent and crash-safe
//! without a marker: if the process dies after writing `personas.json` but
//! before the rename, the next launch still finds the legacy document, takes
//! personas from the file already written and everything else from the legacy
//! document, and finishes. "Legacy wins" would instead overwrite whatever the
//! user changed between the crash and the relaunch.
//!
//! The legacy document is read as a [`Value`] rather than through a
//! `LegacyDocument` struct: the legacy shape *is* the current [`Settings`]
//! shape, so a struct would be a second copy of the same field list, and a
//! field added to one and not the other would silently drop a setting.

use std::fs;

use serde::de::DeserializeOwned;
use serde_json::Value;

use super::{Settings, StorePaths, documents, workspace_split};
use crate::consts::LEGACY_MIGRATED_EXTENSION;
use crate::error::Result;

/// The five keys the legacy document held its collections under.
const SAVED_AGENTS: &str = "savedAgents";
const SAVED_WORKSPACES: &str = "savedWorkspaces";
const PERSONAS: &str = "personas";
const BENCH_AGENTS: &str = "benchAgents";
const RECENT_REPOS: &str = "recentRepos";

/// Migrate the legacy document if one is present, returning the loaded store.
///
/// `Ok(None)` means there was nothing to migrate - either no legacy document,
/// or one that cannot be read. An unreadable one is treated as absent and
/// left under its own name so it can be recovered by hand, which is what the
/// store already did with an undecodable document.
pub fn migrate(paths: &StorePaths) -> Result<Option<Settings>> {
    let legacy_path = paths.legacy();
    let Some(mut document) = documents::read_object(&legacy_path)
    else {
        return Ok(None);
    };
    let object = document.as_object_mut()
                         .expect("read_object yields only objects");

    // Lifted before the remainder is decoded as preferences: `Settings`
    // skips these fields now, so leaving them in would simply drop them.
    let agents = object.remove(SAVED_AGENTS);
    let workspaces = object.remove(SAVED_WORKSPACES);
    let personas = object.remove(PERSONAS);
    let bench = object.remove(BENCH_AGENTS);
    let recent_repos = object.remove(RECENT_REPOS);

    let mut settings = Settings::from_preferences(document);
    settings.saved_agents = existing_or_legacy(&paths.agents(), agents);
    settings.personas = existing_or_legacy(&paths.personas(), personas);
    settings.bench_agents = existing_or_legacy(&paths.bench(), bench);
    settings.recent_repos = existing_or_legacy(&paths.recent_repos(), recent_repos);

    // The workspaces take the split path rather than `existing_or_legacy`:
    // their legacy records are combined, and decoding one into the narrowed
    // `Workspace` would drop its arrangement silently. `read` already applies
    // "the new document wins" to both documents, so the same crash-safety
    // rule holds here without a second implementation of it.
    let split = workspace_split::read(paths);
    settings.workspace_ui = split.ui_state;
    settings.saved_workspaces = if paths.workspaces().exists() {
        split.workspaces
    }
    else {
        let records = match workspaces {
            Some(Value::Array(records)) => records,
            _ => Vec::new(),
        };
        workspace_split::split_records(records, &mut settings.workspace_ui)
    };
    settings.prune_workspace_ui();
    settings.paths = Some(paths.clone());

    settings.persist()?;
    // Only after every write succeeded: a rename before that would lose
    // whatever had not been written yet, with nothing left to retry from.
    fs::rename(&legacy_path,
               legacy_path.with_extension(LEGACY_MIGRATED_EXTENSION))?;

    Ok(Some(settings))
}

/// The collection the new document already holds, or the legacy document's
/// value if that document does not exist yet.
fn existing_or_legacy<T: DeserializeOwned>(path: &std::path::Path, legacy: Option<Value>)
                                           -> Vec<T> {
    if path.exists() {
        return documents::read_collection(path);
    }
    legacy.map(documents::decode_collection).unwrap_or_default()
}

#[cfg(test)]
mod tests;
