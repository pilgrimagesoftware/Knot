//! One-time split of a combined `workspaces.json` into a saved-workspaces
//! document and a per-workspace UI-state document.
//!
//! Contract: `openspec/specs/settings-persistence/spec.md`.
//!
//! An installation written by an earlier build holds both kinds of value in
//! one workspace record: the four the user configured, and the eight the
//! application recorded about how that workspace's window was arranged. On
//! load, when such a record is present, the arrangement is lifted out into
//! [`WorkspaceUiState`] entries and both documents are written.
//!
//! The document is read as a [`Value`] and examined for the UI keys *before*
//! it is decoded, and that ordering is the whole point. Serde ignores unknown
//! fields, so a combined document decodes cleanly into the narrowed
//! [`Workspace`] while dropping every arrangement it held - no error, no
//! warning, and nothing to notice until windows open centred. A migration
//! that ran after decoding could not see what it had already lost.
//!
//! Idempotence follows [`super::legacy`]'s rule rather than a marker of its
//! own: an entry already in the UI-state document wins over the same
//! workspace's combined fields. The UI-state document is written first and
//! the workspaces document second, because rewriting the second is what
//! clears the "combined" condition. A crash between the two leaves a document
//! that still looks combined, and the next launch finds the UI entries
//! already written and finishes without overwriting them.

use std::collections::BTreeMap;

use serde_json::Value;
use uuid::Uuid;

use super::{StorePaths, Workspace, WorkspaceUiState, documents};

/// The eight keys a combined record carries beyond the workspace's own.
///
/// Presence of any one of them is what "combined" means. Listed here rather
/// than derived from [`WorkspaceUiState`]: the detection has to describe the
/// *old* shape, which no longer has a type, and a field later removed from
/// the record must still be recognized in a document that has it.
const UI_KEYS: [&str; 8] = ["layoutMode",
                            "activeAgentIds",
                            "focusedPaneIndex",
                            "splitRatio",
                            "splitRatioSecondary",
                            "showDashboard",
                            "isDetached",
                            "windowBounds"];

/// The workspaces and their UI state, and whether the split has yet to be
/// written back.
pub struct SplitWorkspaces {
    pub workspaces:  Vec<Workspace>,
    pub ui_state:    BTreeMap<Uuid, WorkspaceUiState>,
    /// `true` when a combined document was found, so the caller must write
    /// both documents. `false` on an already-split installation, where a load
    /// must not rewrite either - a migration that ran every launch would put
    /// the roster back in the path of every launch.
    pub needs_write: bool,
}

/// Read the workspaces and UI-state documents, splitting a combined
/// workspaces document if one is present.
pub fn read(paths: &StorePaths) -> SplitWorkspaces {
    let mut ui_state: BTreeMap<Uuid, WorkspaceUiState> =
        documents::read_map(&paths.workspace_ui_state());

    let Some(Value::Array(records)) = read_array(&paths.workspaces())
    else {
        return SplitWorkspaces { workspaces: Vec::new(),
                                 ui_state,
                                 needs_write: false };
    };

    let combined = records.iter().any(has_ui_keys);
    if !combined {
        return SplitWorkspaces { workspaces:
                                     documents::decode_collection(Value::Array(records)),
                                 ui_state,
                                 needs_write: false };
    }

    let workspaces = split_records(records, &mut ui_state);
    SplitWorkspaces { workspaces,
                      ui_state,
                      needs_write: true }
}

/// Lift the UI keys out of each record into `ui_state`, returning the
/// narrowed workspaces.
///
/// Also the path the legacy migration takes: the `savedWorkspaces` it lifts
/// out of `settings.json` is itself in the combined shape, so decoding it
/// straight into [`Workspace`] would drop every arrangement before this
/// module ever saw the document. Both migrations therefore split through
/// here, and an installation still on the legacy document arrives at both new
/// documents rather than at a combined one.
pub fn split_records(records: Vec<Value>, ui_state: &mut BTreeMap<Uuid, WorkspaceUiState>)
                     -> Vec<Workspace> {
    let mut narrowed = Vec::with_capacity(records.len());
    for mut record in records {
        let Some(object) = record.as_object_mut()
        else {
            continue;
        };
        // Lifted before the remainder is decoded: `Workspace` ignores these
        // keys now, so leaving them in would simply drop them.
        let lifted: serde_json::Map<String, Value> =
            UI_KEYS.iter()
                   .filter_map(|key| object.remove(*key).map(|value| ((*key).to_string(), value)))
                   .collect();

        let Ok(workspace) = serde_json::from_value::<Workspace>(record)
        else {
            // A record that cannot be decoded as a workspace has no identity
            // to key its UI state by, and dropping it is what
            // `read_collection` already does with an undecodable record.
            continue;
        };
        // The already-written entry wins, so a relaunch after a crash between
        // the two writes keeps what the user has done since.
        if let std::collections::btree_map::Entry::Vacant(slot) = ui_state.entry(workspace.id) {
            // Field by field tolerant, like any other load: a record written
            // before one of these fields existed takes that field's default
            // rather than failing the entry.
            slot.insert(serde_json::from_value(Value::Object(lifted)).unwrap_or_default());
        }
        narrowed.push(workspace);
    }
    narrowed
}

/// Whether `record` still carries any of the keys that moved out.
///
/// Any one of them, not all eight: a record written before a UI field existed
/// carries only the others, and is just as combined.
fn has_ui_keys(record: &Value) -> bool {
    record.as_object()
          .is_some_and(|object| UI_KEYS.iter().any(|key| object.contains_key(*key)))
}

/// Read `path` as a JSON array, or `None` on the same terms
/// [`documents::read_collection`] treats as empty.
fn read_array(path: &std::path::Path) -> Option<Value> {
    let bytes = std::fs::read(path).ok()?;
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(value) if value.is_array() => Some(value),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
