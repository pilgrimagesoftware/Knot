//! Reading and writing one settings document.
//!
//! Contract: `openspec/specs/settings-persistence/spec.md`.
//!
//! Every document is written atomically and read tolerantly. Writing goes to
//! a temporary file beside the target and renames it into place, so a reader
//! observes either the previous document or the new one. Reading yields that
//! document's defaults - empty for a collection or a map - for anything it
//! cannot make sense of, so one unreadable document never stops the rest of
//! the store, or the app, from loading.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::consts::DOCUMENT_TEMP_EXTENSION;
use crate::error::Result;

/// Write `bytes` to `path`, creating the parent directory as needed.
///
/// Written to a temporary file in the same directory and renamed into place.
/// Nearly every mutating helper on [`super::Settings`] persists immediately,
/// so this runs on most user actions; a truncating write interrupted by a
/// crash or a power loss would leave unparseable JSON, and the next launch
/// would silently fall back to defaults - losing whatever that document held
/// with no way back. `rename` within one directory is atomic on macOS and
/// Linux, so a reader sees either the old document or the new one.
pub fn write(path: &Path, bytes: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    // Same directory as the target: `rename` is only atomic within one
    // filesystem, and a temp dir may be on another.
    let temporary = path.with_extension(DOCUMENT_TEMP_EXTENSION);
    fs::write(&temporary, bytes)?;
    match fs::rename(&temporary, path) {
        Ok(()) => Ok(()),
        Err(error) => {
            // Leaving the temp file behind would shadow the next attempt's
            // write with a stale document.
            let _ = fs::remove_file(&temporary);
            Err(error.into())
        }
    }
}

/// Serialize `records` as a bare JSON array and write it to `path`.
///
/// A bare array rather than an object wrapper: the document's name already
/// says what it holds, so a wrapper would only add a key to get wrong and a
/// nesting level to hand-edit past.
pub fn write_collection<T: Serialize>(path: &Path, records: &[T]) -> Result<()> {
    write(path, &serde_json::to_string_pretty(records)?)
}

/// Serialize `entries` as a JSON object keyed by the map's key and write it
/// to `path`.
///
/// A map rather than the bare array [`write_collection`] produces, because
/// the document it serves is a lookup table rather than a collection the user
/// browses: every read is by key, and an array would admit duplicate keys the
/// reader would then have to rule out.
pub fn write_map<K: ToString, V: Serialize>(path: &Path, entries: &BTreeMap<K, V>) -> Result<()> {
    let object: serde_json::Map<String, Value> =
        entries.iter()
               .map(|(key, value)| (key.to_string(), serde_json::to_value(value)))
               .map(|(key, value)| value.map(|value| (key, value)))
               .collect::<std::result::Result<_, _>>()?;
    write(path, &serde_json::to_string_pretty(&object)?)
}

/// Read `path` as a JSON object keyed by `K`, decoding each entry on its own
/// and dropping the ones that fail.
///
/// The same tolerance [`read_collection`] gives an array, for the same
/// reason: a missing, unreadable or malformed document yields an empty map,
/// and one entry a build can no longer decode - or one whose key is not a
/// valid `K` - costs only what that entry held.
pub fn read_map<K, V>(path: &Path) -> BTreeMap<K, V>
    where K: FromStr + Ord,
          V: DeserializeOwned {
    let Ok(bytes) = fs::read(path)
    else {
        return BTreeMap::new();
    };
    let Ok(Value::Object(entries)) = serde_json::from_slice::<Value>(&bytes)
    else {
        return BTreeMap::new();
    };
    decode_map(entries)
}

/// Decode an already-parsed object as a map, on the same terms as
/// [`read_map`].
///
/// The split of a combined workspaces document reads its entries out of a
/// parsed document rather than off disk, and must apply exactly these rules.
pub fn decode_map<K, V>(entries: serde_json::Map<String, Value>) -> BTreeMap<K, V>
    where K: FromStr + Ord,
          V: DeserializeOwned {
    entries.into_iter()
           .filter_map(|(key, value)| {
               let key = K::from_str(&key).ok()?;
               Some((key, serde_json::from_value(value).ok()?))
           })
           .collect()
}

/// Read `path` as a JSON object, or `None` if it is missing, unreadable, not
/// JSON, or not an object.
pub fn read_object(path: &Path) -> Option<Value> {
    let bytes = fs::read(path).ok()?;
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(value) if value.is_object() => Some(value),
        _ => None,
    }
}

/// Read `path` as a bare JSON array, decoding each record on its own and
/// dropping the ones that fail.
///
/// A missing, unreadable, non-JSON or non-array document yields an empty
/// collection, so a corrupt document costs only what it held. Decoding per
/// record rather than the whole array means one record written by a build
/// that has since changed a type cannot take its neighbours with it.
pub fn read_collection<T: DeserializeOwned>(path: &Path) -> Vec<T> {
    let Ok(bytes) = fs::read(path)
    else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_slice::<Value>(&bytes)
    else {
        return Vec::new();
    };
    decode_collection(value)
}

/// Decode an already-parsed value as a collection, on the same terms as
/// [`read_collection`]: anything that is not an array yields empty, and a
/// record that fails to decode is dropped rather than taking its neighbours
/// with it.
///
/// The migration in [`super::legacy`] reads its collections out of a parsed
/// document rather than off disk, and must apply exactly these rules.
pub fn decode_collection<T: DeserializeOwned>(value: Value) -> Vec<T> {
    let Value::Array(records) = value
    else {
        return Vec::new();
    };
    records.into_iter()
           .filter_map(|record| serde_json::from_value(record).ok())
           .collect()
}

#[cfg(test)]
mod tests;
