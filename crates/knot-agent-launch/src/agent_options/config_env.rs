//! Codex's and opencode's options, as the config object each adapter reads
//! from an environment variable - the one the standing instructions go in
//! (see `crate::instructions`).
//!
//! `codex-acp` spawns `codex app-server` without passing its own argv on,
//! so a Codex CLI flag reaches Codex only as a config override:
//! `CODEX_CONFIG` is sent on every `thread/start` and `thread/resume`.
//! `opencode acp` takes neither `--model` nor `--agent`, but reads both from
//! `OPENCODE_CONFIG_CONTENT`.

use serde_json::{Map, Value};

use super::AdapterOptions;
use super::flags::{Given, split};
use crate::consts::{
    CODEX_CONFIG_ENV, CODEX_FEATURES_KEY, CODEX_OPTION_FLAGS, CODEX_PROFILE_KEY, MODEL_CONFIG_KEY,
    OPENCODE_CONFIG_ENV, OPENCODE_DEFAULT_AGENT_KEY, OPENCODE_OPTION_FLAGS,
};

/// Codex's options: `-c key=value` as that override, `--model` and
/// `--profile` as theirs, `--enable`/`--disable` as a feature switch.
pub(super) fn codex_options(words: Vec<String>) -> AdapterOptions {
    let (given, mut ignored) = split(words, &CODEX_OPTION_FLAGS);
    let mut config = Map::new();
    for flag in given {
        let Given { spec, value } = flag;
        let Some(value) = value
        else {
            continue;
        };
        match spec.long {
            "config" => match value.split_once('=') {
                Some((key, raw)) if !key.is_empty() => set_path(&mut config, key, parse_value(raw)),
                _ => ignored.push(format!("-c {value}")),
            },
            "model" => set_path(&mut config, MODEL_CONFIG_KEY, Value::String(value)),
            "profile" => set_path(&mut config, CODEX_PROFILE_KEY, Value::String(value)),
            "enable" => set_path(&mut config,
                                 &format!("{CODEX_FEATURES_KEY}.{value}"),
                                 Value::Bool(true)),
            "disable" => set_path(&mut config,
                                  &format!("{CODEX_FEATURES_KEY}.{value}"),
                                  Value::Bool(false)),
            _ => {}
        }
    }
    AdapterOptions { env_config: (!config.is_empty()).then_some((CODEX_CONFIG_ENV, config)),
                     ignored,
                     ..AdapterOptions::default() }
}

/// opencode's options: `--model` and `--agent` into its config, the logging
/// and plugin switches onto `opencode acp`'s own command line.
pub(super) fn opencode_options(words: Vec<String>) -> AdapterOptions {
    let (given, ignored) = split(words, &OPENCODE_OPTION_FLAGS);
    let mut config = Map::new();
    let mut args = Vec::new();
    for flag in given {
        match (flag.spec.long, &flag.value) {
            ("model", Some(model)) => {
                config.insert(MODEL_CONFIG_KEY.to_owned(), Value::String(model.clone()));
            }
            ("agent", Some(agent)) => {
                config.insert(OPENCODE_DEFAULT_AGENT_KEY.to_owned(),
                              Value::String(agent.clone()));
            }
            _ => args.extend(flag.words()),
        }
    }
    AdapterOptions { env_config: (!config.is_empty()).then_some((OPENCODE_CONFIG_ENV, config)),
                     args,
                     ignored,
                     ..AdapterOptions::default() }
}

/// A `-c` value as Codex reads it: TOML, falling back to the raw text as a
/// string. JSON covers the TOML a flag is written with - numbers, booleans,
/// quoted strings, arrays of those - and the fallback covers a bare word.
fn parse_value(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_owned()))
}

/// Sets dotted `path` in `config`, creating the tables along it and
/// replacing a non-table in the way, as Codex does for `-c a.b=value`.
fn set_path(config: &mut Map<String, Value>, path: &str, value: Value) {
    let mut keys = path.split('.').peekable();
    let mut table = config;
    while let Some(key) = keys.next() {
        if keys.peek().is_none() {
            table.insert(key.to_owned(), value);
            return;
        }
        let entry = table.entry(key.to_owned())
                         .or_insert_with(|| Value::Object(Map::new()));
        if !entry.is_object() {
            *entry = Value::Object(Map::new());
        }
        let Value::Object(next) = entry
        else {
            return;
        };
        table = next;
    }
}

/// `source`'s keys merged into `target`, table by table: a nested object is
/// merged rather than replaced, so `-c features.x=true` keeps the features
/// Knot's environment already switched on.
pub(super) fn merge_deep(target: &mut Map<String, Value>, source: Map<String, Value>) {
    for (key, value) in source {
        match (target.get_mut(&key), value) {
            (Some(Value::Object(existing)), Value::Object(value)) => merge_deep(existing, value),
            (_, value) => {
                target.insert(key, value);
            }
        }
    }
}
