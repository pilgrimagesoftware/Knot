//! A non-shell agent's user options - the Coding tab's "Options" for its
//! type - in the form its ACP adapter accepts them.
//!
//! Contract: `openspec/specs/agent-launch-command/spec.md`'s "User options
//! for an ACP-launched agent" requirement.
//!
//! The terminal builder that appended these to the agent's command line went
//! with `acp-only-agent-launch`, and nothing took its place, so options set
//! in Settings were saved and never used (#501). `claude-agent-acp` spawns
//! `claude` itself and takes extra CLI flags only as
//! `_meta.claudeCode.options.extraArgs` on `session/new` and
//! `session/load`. The permission flags are the exception: the adapter
//! always passes its own `--permission-mode`, so they become the mode the
//! session is moved to once it opens, as the Coding tab's user expects of a
//! CLI flag.

use serde_json::{Map, Value, json};

use crate::DefaultMode;
use crate::consts::{
    CLAUDE_ADAPTER_FLAGS, CLAUDE_BYPASS_MODE, CLAUDE_MODE_CONFIG_ID, CLAUDE_PERMISSION_MODE_FLAG,
    CLAUDE_PERMISSION_MODES, CLAUDE_SKIP_PERMISSIONS_FLAG,
};

/// What an agent type's user options ask of its adapter's session.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AdapterOptions {
    /// The `_meta` to open the session with, or `None` when there is
    /// nothing to send.
    pub session_meta: Option<Value>,
    /// The permission mode the options ask the session to start in.
    pub mode:         Option<DefaultMode>,
    /// Words that were not forwarded, for the caller to report: a type
    /// whose adapter takes no options, a flag the adapter owns, a short
    /// flag or a stray word, or the whole text when its quoting is
    /// unbalanced.
    pub ignored:      Vec<String>,
}

/// `options`, the free text the user set for `agent_type`, as its adapter
/// takes it. Pure.
pub fn adapter_options(agent_type: &str, options: &str) -> AdapterOptions {
    if options.trim().is_empty() {
        return AdapterOptions::default();
    }
    match agent_type {
        "claude" => claude_options(options),
        // No adapter here has a confirmed way in yet; say so rather than
        // drop the options silently.
        _ => AdapterOptions { ignored: vec![options.trim().to_owned()],
                              ..AdapterOptions::default() },
    }
}

/// Claude's options: permission flags to a mode, the rest to `extraArgs`,
/// which the SDK writes back out as `--name value`, or `--name` for a
/// `null`. A repeated flag keeps its last value - `extraArgs` is a map.
fn claude_options(options: &str) -> AdapterOptions {
    let Ok(words) = shell_words::split(options)
    else {
        return AdapterOptions { ignored: vec![options.trim().to_owned()],
                                ..AdapterOptions::default() };
    };
    let mut extra_args = Map::new();
    let mut mode = None;
    let mut ignored = Vec::new();
    let mut words = words.into_iter().peekable();
    while let Some(word) = words.next() {
        let Some(flag) = word.strip_prefix("--").filter(|flag| !flag.is_empty())
        else {
            ignored.push(word);
            continue;
        };
        let (name, inline) = match flag.split_once('=') {
            Some((name, value)) => (name, Some(value.to_owned())),
            None => (flag, None),
        };
        if name == CLAUDE_SKIP_PERMISSIONS_FLAG && inline.is_none() {
            mode = Some(claude_mode(CLAUDE_BYPASS_MODE));
            continue;
        }
        // Whatever follows a flag, up to the next flag, is its value.
        let value = inline.or_else(|| words.next_if(|next| !next.starts_with('-')));
        if name == CLAUDE_PERMISSION_MODE_FLAG {
            match value.as_deref().and_then(permission_mode) {
                Some(id) => mode = Some(claude_mode(id)),
                None => ignored.push(word),
            }
        }
        else if CLAUDE_ADAPTER_FLAGS.contains(&name) {
            ignored.push(word);
        }
        else {
            extra_args.insert(name.to_owned(), value.map_or(Value::Null, Value::String));
        }
    }
    let session_meta = (!extra_args.is_empty())
        .then(|| json!({ "claudeCode": { "options": { "extraArgs": extra_args } } }));
    AdapterOptions { session_meta,
                     mode,
                     ignored }
}

/// The adapter's mode id for a `--permission-mode` value, in any spelling
/// Claude Code accepts, or `None` for one it does not know.
fn permission_mode(value: &str) -> Option<&'static str> {
    let value = value.trim().to_lowercase();
    CLAUDE_PERMISSION_MODES.iter()
                           .find(|(alias, _)| *alias == value)
                           .map(|(_, id)| *id)
}

fn claude_mode(value: &'static str) -> DefaultMode {
    DefaultMode { config_id: CLAUDE_MODE_CONFIG_ID,
                  value }
}

#[cfg(test)]
mod tests;
