//! Where an ACP adapter takes Knot's standing instructions - the knot
//! instructions and the agent's persona - so the conversation's first turn
//! is the registration request alone (#534).
//!
//! Sent as a user turn, the instructions were the first thing every panel
//! showed, a wall of text the user never wrote. And a turn is history: a
//! resumed session got them again only by replaying it. Each adapter that
//! has a system or developer channel takes them there instead, on every
//! launch, resume included. One with no known channel keeps the old first
//! turn.
//!
//! Contract: `openspec/specs/agent-launch-command/spec.md`, "Standing
//! instructions through the agent's system channel".

use std::io;
use std::path::{Path, PathBuf};

use knot_core::Persona;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::consts::{
    CODEX_CONFIG_ENV, CODEX_DEVELOPER_INSTRUCTIONS_KEY, INSTRUCTIONS_FILES_DIR,
    OPENCODE_CONFIG_ENV, OPENCODE_INSTRUCTIONS_KEY,
};
use crate::escape::persona_prompt;
use crate::registration::knot_instructions;

/// How an agent type's adapter takes standing instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionCarrier {
    /// `_meta.systemPrompt.append` on `session/new` and `session/load`,
    /// which `claude-agent-acp` appends to its `claude_code` preset. The
    /// object form: a string there replaces the preset outright.
    SessionMeta,
    /// `developer_instructions` in the adapter's `CODEX_CONFIG`, which
    /// `codex-acp` sends as config overrides on `thread/start` and
    /// `thread/resume` alike. Not a `-c` argument, as the Swift app used
    /// with the Codex CLI: `codex-acp` spawns `codex app-server` with no
    /// arguments of its own.
    CodexConfigEnv,
    /// A file named in the adapter's `OPENCODE_CONFIG_CONTENT`
    /// `instructions`, which opencode appends to the system prompt it
    /// rebuilds on every step. A file because that key takes paths, not
    /// text.
    OpencodeConfigEnv,
    /// Ahead of the registration request, on a fresh session only.
    FirstTurn,
}

/// The carrier for `agent_type`. [`InstructionCarrier::FirstTurn`] for any
/// type without a system channel confirmed against its adapter's source -
/// Gemini and Copilot today, and any type added later until someone checks.
pub fn instruction_carrier(agent_type: &str) -> InstructionCarrier {
    match agent_type {
        "claude" => InstructionCarrier::SessionMeta,
        "codex" => InstructionCarrier::CodexConfigEnv,
        "opencode" => InstructionCarrier::OpencodeConfigEnv,
        _ => InstructionCarrier::FirstTurn,
    }
}

/// The standing instructions for `agent_id`: the knot instructions, then
/// the persona when it has any. One line, so the first-turn fallback keeps
/// the registration prompt's single-line guarantee.
pub fn standing_instructions(agent_id: Uuid, persona: Option<&Persona>) -> String {
    let mut text = knot_instructions(agent_id);
    if let Some(persona) = persona_prompt(persona) {
        text.push(' ');
        text.push_str(&persona);
    }
    text
}

/// A file an adapter reads its instructions from, written before it is
/// spawned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionsFile {
    pub path:     PathBuf,
    pub contents: String,
}

impl InstructionsFile {
    /// Writes the file, creating its directory. Blocking I/O: call it off
    /// the render path.
    pub fn write(&self) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, &self.contents)
    }
}

/// What one launch sends its standing instructions through. Everything but
/// the first-turn text applies on every launch, resume included.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InstructionDelivery {
    /// Merged into the session's `_meta` on `session/new` and
    /// `session/load`.
    pub session_meta: Option<Value>,
    /// Set on the adapter subprocess.
    pub env:          Vec<(String, String)>,
    /// To write before the adapter is spawned; `env` names it.
    pub file:         Option<InstructionsFile>,
    /// To lead the registration request with, on a fresh session.
    pub first_turn:   Option<String>,
}

impl InstructionDelivery {
    /// The same instructions moved to the first turn, for when `file`
    /// could not be written: an adapter pointed at a missing file would
    /// start with no instructions at all.
    #[must_use]
    pub fn into_first_turn(self) -> Self {
        match self.file {
            Some(file) => Self { first_turn: Some(file.contents),
                                 ..Self::default() },
            None => self,
        }
    }
}

/// What [`instruction_delivery`] needs to know about one launch.
pub struct DeliveryRequest<'a> {
    pub carrier:      InstructionCarrier,
    pub agent_id:     Uuid,
    /// [`standing_instructions`]' text.
    pub instructions: String,
    /// Where per-agent files go - the app's cache directory - or `None`
    /// when there is none, which sends a file carrier's text on the first
    /// turn instead.
    pub cache_dir:    Option<&'a Path>,
    /// The value an environment variable already has in Knot's own
    /// environment, which the adapter would otherwise inherit.
    pub inherited:    &'a dyn Fn(&str) -> Option<String>,
}

/// How `request`'s instructions reach its adapter. Pure.
///
/// An environment carrier merges into the variable's inherited value rather
/// than replacing it, so configuration a user set there themselves still
/// reaches the agent.
pub fn instruction_delivery(request: DeliveryRequest<'_>) -> InstructionDelivery {
    let DeliveryRequest { carrier,
                          agent_id,
                          instructions,
                          cache_dir,
                          inherited, } = request;
    match carrier {
        InstructionCarrier::SessionMeta => {
            InstructionDelivery { session_meta: Some(json!({ "systemPrompt": { "append": instructions } })),
                                  ..InstructionDelivery::default() }
        }
        InstructionCarrier::CodexConfigEnv => {
            let mut config = inherited_object(inherited(CODEX_CONFIG_ENV));
            let text = match config.get(CODEX_DEVELOPER_INSTRUCTIONS_KEY) {
                Some(Value::String(own)) if !own.is_empty() => format!("{own}\n\n{instructions}"),
                _ => instructions,
            };
            config.insert(CODEX_DEVELOPER_INSTRUCTIONS_KEY.to_owned(),
                          Value::String(text));
            InstructionDelivery { env: vec![(CODEX_CONFIG_ENV.to_owned(),
                                             Value::Object(config).to_string())],
                                  ..InstructionDelivery::default() }
        }
        InstructionCarrier::OpencodeConfigEnv => {
            let Some(cache_dir) = cache_dir
            else {
                return InstructionDelivery { first_turn: Some(instructions),
                                             ..InstructionDelivery::default() };
            };
            let path = cache_dir.join(INSTRUCTIONS_FILES_DIR)
                                .join(format!("{agent_id}.md"));
            let mut config = inherited_object(inherited(OPENCODE_CONFIG_ENV));
            let mut files = match config.remove(OPENCODE_INSTRUCTIONS_KEY) {
                Some(Value::Array(files)) => files,
                _ => Vec::new(),
            };
            files.push(Value::String(path.to_string_lossy().into_owned()));
            config.insert(OPENCODE_INSTRUCTIONS_KEY.to_owned(), Value::Array(files));
            InstructionDelivery { env: vec![(OPENCODE_CONFIG_ENV.to_owned(),
                                             Value::Object(config).to_string())],
                                  file: Some(InstructionsFile { path,
                                                                contents: instructions }),
                                  ..InstructionDelivery::default() }
        }
        InstructionCarrier::FirstTurn => InstructionDelivery { first_turn: Some(instructions),
                                                               ..InstructionDelivery::default() },
    }
}

/// `value` as a JSON object, or an empty one when it is unset or is not an
/// object - the adapter would reject such a value itself.
fn inherited_object(value: Option<String>) -> Map<String, Value> {
    match value.and_then(|text| serde_json::from_str(&text).ok()) {
        Some(Value::Object(object)) => object,
        _ => Map::new(),
    }
}

/// `extra`'s top-level keys merged over `base`'s, for a session opening
/// with both the user's options and the standing instructions in `_meta`.
/// The two own disjoint keys (`claudeCode`, `systemPrompt`), so a shallow
/// merge loses nothing.
pub fn merge_session_meta(base: Option<Value>, extra: Option<Value>) -> Option<Value> {
    match (base, extra) {
        (Some(Value::Object(mut base)), Some(Value::Object(extra))) => {
            base.extend(extra);
            Some(Value::Object(base))
        }
        (base, None) => base,
        (None, extra) => extra,
        // A non-object `_meta` is not something either side builds.
        (_, extra) => extra,
    }
}

#[cfg(test)]
mod tests;
