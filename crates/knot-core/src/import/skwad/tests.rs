//! Tests for the Skwad preferences reader.
//!
//! The fixture plist is built here rather than committed, from the exact
//! record shapes a real `com.kochava.skwad.plist` holds - JSON documents
//! stored as `data` values under the four collection keys.

use std::path::Path;

use plist::{Dictionary, Value};
use tempfile::TempDir;

use super::{SkwadSource, read_from};
use crate::consts::{
    SKWAD_AGENTS_KEY, SKWAD_BENCH_AGENTS_KEY, SKWAD_PERSONAS_KEY, SKWAD_WORKSPACES_KEY,
};
use crate::import::result::UnreadableReason;

const WORKSPACE_ID: &str = "AFD849AC-66CE-4A2F-9B20-D7F6676AC1E5";
const AGENT_ID: &str = "736089E6-98F5-47EE-967F-5D2C6F517ECB";
const PERSONA_ID: &str = "079CF92D-B600-4EF5-87D1-3A26C830322B";
const BENCH_ID: &str = "EF4A51C7-281F-4425-9353-82D2D30F48CC";

fn workspaces_json() -> String {
    format!(r##"[{{"splitRatio":0.5,"agentIds":["{AGENT_ID}"],"activeAgentIds":["{AGENT_ID}"],
             "id":"{WORKSPACE_ID}","showDashboard":false,"focusedPaneIndex":0,
             "isDetached":false,"colorHex":"#46A857","name":"WIP","layoutMode":"single"}}]"##)
}

fn agents_json() -> String {
    format!(r#"[{{"isCompanion":false,"folder":"/Users/someone/WIP/thing","id":"{AGENT_ID}",
             "avatar":"🛠️","agentType":"claude","name":"right-sizing",
             "personaId":"{PERSONA_ID}"}}]"#)
}

fn personas_json() -> String {
    format!(r#"[{{"id":"{PERSONA_ID}","instructions":"Be brief.","type":"user","name":"Terse",
             "state":"enabled"}}]"#)
}

fn bench_json() -> String {
    format!(r#"[{{"avatar":"🔥","folder":"/Users/someone/WIP/thing","agentType":"claude",
             "personaId":"{PERSONA_ID}","name":"On-call Questions","id":"{BENCH_ID}"}}]"#)
}

/// Write a plist holding the given collection blobs, the way Skwad stores
/// them: JSON text inside a `data` value.
fn write_plist(dir: &TempDir, entries: &[(&str, &str)]) -> std::path::PathBuf {
    let mut prefs = Dictionary::new();
    // A scalar Skwad also stores, present so the fixture is not just the four
    // keys the reader wants.
    prefs.insert("keepInMenuBar".into(), Value::Boolean(true));
    for (key, json) in entries {
        prefs.insert((*key).into(), Value::Data(json.as_bytes().to_vec()));
    }
    let path = dir.path().join("com.kochava.skwad.plist");
    Value::Dictionary(prefs).to_file_binary(&path)
                            .expect("fixture plist written");
    path
}

fn full_fixture(dir: &TempDir) -> std::path::PathBuf {
    write_plist(dir,
                &[(SKWAD_WORKSPACES_KEY, &workspaces_json()),
                  (SKWAD_AGENTS_KEY, &agents_json()),
                  (SKWAD_PERSONAS_KEY, &personas_json()),
                  (SKWAD_BENCH_AGENTS_KEY, &bench_json())])
}

#[test]
fn every_collection_decodes_into_knots_own_records() {
    let dir = TempDir::new().expect("temp dir");

    let source = read_from(&full_fixture(&dir));

    assert_eq!(source.workspaces.len(), 1);
    assert_eq!(source.workspaces[0].name, "WIP");
    assert_eq!(source.agents.len(), 1);
    assert_eq!(source.agents[0].name, "right-sizing");
    assert_eq!(source.personas.len(), 1);
    assert_eq!(source.personas[0].name, "Terse");
    assert_eq!(source.bench_agents.len(), 1);
    assert_eq!(source.bench_agents[0].name, "On-call Questions");
    assert!(source.unreadable.is_empty());
}

/// The records are ports of each other, so the references have to line up
/// without translation.
#[test]
fn the_decoded_records_still_reference_each_other() {
    let dir = TempDir::new().expect("temp dir");

    let source = read_from(&full_fixture(&dir));

    let workspace = &source.workspaces[0];
    let agent = &source.agents[0];
    assert_eq!(workspace.agent_ids, vec![agent.id]);
    assert_eq!(agent.persona_id, Some(source.personas[0].id));
    assert_eq!(source.bench_agents[0].folder, agent.folder);
}

/// Skwad may not be installed: that is nothing to import, not an error.
#[test]
fn an_absent_domain_yields_nothing_rather_than_an_error() {
    let dir = TempDir::new().expect("temp dir");

    let source = read_from(&dir.path().join("com.kochava.skwad.plist"));

    assert_eq!(source, SkwadSource::default());
    assert!(source.is_empty());
}

#[test]
fn a_file_that_is_not_a_plist_yields_nothing() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("junk.plist");
    std::fs::write(&path, b"not a plist at all").expect("fixture written");

    assert!(read_from(&path).is_empty());
}

/// A domain that exists but holds none of the collection keys - a Skwad that
/// was launched and never used.
#[test]
fn an_absent_key_contributes_nothing() {
    let dir = TempDir::new().expect("temp dir");
    let path = write_plist(&dir, &[(SKWAD_PERSONAS_KEY, &personas_json())]);

    let source = read_from(&path);

    assert_eq!(source.personas.len(), 1);
    assert!(source.workspaces.is_empty());
    assert!(source.agents.is_empty());
    assert!(source.bench_agents.is_empty());
    assert!(source.unreadable.is_empty(),
            "an absent key is not a failure");
}

#[test]
fn a_blob_that_is_not_json_is_reported_not_fatal() {
    let dir = TempDir::new().expect("temp dir");
    let path = write_plist(&dir,
                           &[(SKWAD_WORKSPACES_KEY, "{ not json"),
                             (SKWAD_PERSONAS_KEY, &personas_json())]);

    let source = read_from(&path);

    assert!(source.workspaces.is_empty());
    assert_eq!(source.personas.len(),
               1,
               "the readable collection still arrives");
    assert_eq!(source.unreadable.len(), 1);
    assert_eq!(source.unreadable[0].reason, UnreadableReason::Malformed);
}

/// One malformed record must not empty the collection around it.
#[test]
fn one_malformed_record_among_good_ones_is_named_and_dropped() {
    let dir = TempDir::new().expect("temp dir");
    let broken = format!(
                         r#"[{{"id":"{PERSONA_ID}","instructions":"Be brief.","type":"user","name":"Terse",
             "state":"enabled"}},
           {{"id":"not-a-uuid","name":"Broken","instructions":"x"}}]"#
    );
    let path = write_plist(&dir, &[(SKWAD_PERSONAS_KEY, &broken)]);

    let source = read_from(&path);

    assert_eq!(source.personas.len(), 1);
    assert_eq!(source.personas[0].name, "Terse");
    assert_eq!(source.unreadable.len(), 1);
    assert_eq!(source.unreadable[0].name, "Broken", "named by its own name");
}

/// Task 3.4: reading must leave Skwad's preferences byte-for-byte unchanged.
#[test]
fn reading_leaves_the_preferences_file_untouched() {
    let dir = TempDir::new().expect("temp dir");
    let path = full_fixture(&dir);
    let before = std::fs::read(&path).expect("read before");

    let _ = read_from(&path);
    let _ = read_from(&path);

    assert_eq!(std::fs::read(&path).expect("read after"), before);
}

#[test]
fn the_preferences_path_names_the_skwad_domain() {
    let path = super::preferences_path().expect("a home directory");

    assert!(path.ends_with(Path::new("Library/Preferences/com.kochava.skwad.plist")),
            "unexpected path: {}",
            path.display());
}
