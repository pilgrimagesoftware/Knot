//! Reading Skwad's preferences.
//!
//! Contract: `openspec/specs/data-import/spec.md` - "Skwad source and format".
//!
//! Skwad is Knot's predecessor, and Knot's `SavedAgent`, `Workspace`,
//! `Persona` and `BenchAgent` are ports of its Swift records serialized with
//! the same camelCase keys. Its collections are JSON documents stored as
//! `data` values inside the `com.kochava.skwad` preferences plist, so reading
//! them is a decode and a filter, not a migration.
//!
//! Strictly read-only: nothing here opens the plist for writing, so a running
//! Skwad is unaffected and a user can keep using it while they move across.

use std::path::{Path, PathBuf};

use directories::BaseDirs;
use plist::Value;
use serde::de::DeserializeOwned;

use super::result::{Unreadable, UnreadableReason};
use crate::consts::{
    SKWAD_AGENTS_KEY, SKWAD_BENCH_AGENTS_KEY, SKWAD_PERSONAS_KEY, SKWAD_PREFERENCES_DOMAIN,
    SKWAD_WORKSPACES_KEY,
};
use crate::settings::{BenchAgent, Persona, SavedAgent, Workspace};

#[cfg(test)]
mod tests;

/// Everything a Skwad installation offers, already decoded into Knot's own
/// record types.
///
/// An absent domain, an absent key, or an undecodable blob all yield empty
/// collections rather than an error: Skwad may simply not be installed, and
/// `data-import` requires that case to be silent.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct SkwadSource {
    pub workspaces:   Vec<Workspace>,
    pub agents:       Vec<SavedAgent>,
    pub personas:     Vec<Persona>,
    pub bench_agents: Vec<BenchAgent>,
    /// Records that were present but could not be decoded, named so one bad
    /// entry is visible rather than silently missing.
    pub unreadable:   Vec<Unreadable>,
}

impl SkwadSource {
    /// Whether there is anything at all to offer.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.workspaces.is_empty()
        && self.agents.is_empty()
        && self.personas.is_empty()
        && self.bench_agents.is_empty()
    }
}

/// Where Skwad's preferences live for the current user, if a home directory
/// can be resolved at all.
#[must_use]
pub fn preferences_path() -> Option<PathBuf> {
    let home = BaseDirs::new()?.home_dir().to_path_buf();
    Some(home.join("Library")
             .join("Preferences")
             .join(format!("{SKWAD_PREFERENCES_DOMAIN}.plist")))
}

/// Read the installed Skwad's preferences. Empty when there are none.
#[must_use]
pub fn read() -> SkwadSource {
    preferences_path().map(|path| read_from(&path))
                      .unwrap_or_default()
}

/// Read a specific preferences file.
///
/// A missing or unparsable file is nothing to import, not an error - the same
/// answer a machine without Skwad gives.
#[must_use]
pub fn read_from(path: &Path) -> SkwadSource {
    let Ok(Value::Dictionary(prefs)) = Value::from_file(path)
    else {
        return SkwadSource::default();
    };

    let mut source = SkwadSource::default();
    source.workspaces = decode_collection(&prefs, SKWAD_WORKSPACES_KEY, &mut source.unreadable);
    source.agents = decode_collection(&prefs, SKWAD_AGENTS_KEY, &mut source.unreadable);
    source.personas = decode_collection(&prefs, SKWAD_PERSONAS_KEY, &mut source.unreadable);
    source.bench_agents = decode_collection(&prefs, SKWAD_BENCH_AGENTS_KEY, &mut source.unreadable);
    source
}

/// Decode one collection key into `T`, one record at a time.
///
/// The blob is read as a list of raw JSON values first so a single record that
/// does not fit Knot's shape is reported and dropped rather than emptying the
/// whole collection - `data-import` forbids one bad record from abandoning the
/// rest.
fn decode_collection<T: DeserializeOwned>(prefs: &plist::Dictionary, key: &str,
                                          unreadable: &mut Vec<Unreadable>)
                                          -> Vec<T> {
    let Some(bytes) = prefs.get(key).and_then(Value::as_data)
    else {
        return Vec::new();
    };
    let Ok(raw) = serde_json::from_slice::<Vec<serde_json::Value>>(bytes)
    else {
        unreadable.push(Unreadable::new(key, UnreadableReason::Malformed));
        return Vec::new();
    };

    let mut records = Vec::with_capacity(raw.len());
    for (index, value) in raw.into_iter().enumerate() {
        let label = record_label(key, index, &value);
        match serde_json::from_value(value) {
            Ok(record) => records.push(record),
            Err(_) => unreadable.push(Unreadable::new(label, UnreadableReason::Malformed)),
        }
    }
    records
}

/// How an undecodable record identifies itself: its own `name` when it has
/// one, otherwise its position in the collection.
fn record_label(key: &str, index: usize, value: &serde_json::Value) -> String {
    value.get("name")
         .and_then(serde_json::Value::as_str)
         .map_or_else(|| format!("{key}[{index}]"), ToString::to_string)
}
