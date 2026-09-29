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
//!
//! The other adapters take options their own way: Codex and opencode as the
//! config object they read from an environment variable, Gemini on its
//! command line. Each accepts only the flags its table names (see
//! [`flags`]). Copilot's CLI has not been checked, so its options are
//! reported as not passed.

mod argv;
mod config_env;
mod flags;

use serde_json::{Map, Value, json};

pub(crate) use self::flags::FlagSpec;
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
    /// Arguments for the adapter's command line, after its own.
    pub args:         Vec<String>,
    /// Config overrides for an adapter that reads a JSON object from an
    /// environment variable: the variable, and the keys to set in it. See
    /// [`Self::layered_env`].
    pub env_config:   Option<(&'static str, Map<String, Value>)>,
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
        "codex" => with_words(options, config_env::codex_options),
        "opencode" => with_words(options, config_env::opencode_options),
        "gemini" => with_words(options, argv::gemini_options),
        // No confirmed way in - Copilot's CLI is not checked yet - so say
        // so rather than drop the options silently.
        _ => all_ignored(options),
    }
}

impl AdapterOptions {
    /// `name`'s value in the adapter's environment, before the standing
    /// instructions are added: `inherited`, Knot's own value, with these
    /// options' config merged over it. `inherited` as it is for any other
    /// variable.
    ///
    /// Options sit between the two on purpose. They override what the
    /// environment set, as a CLI flag overrides a config file, and the
    /// instructions merge into the result afterwards, so no option can
    /// displace them.
    pub fn layered_env(&self, name: &str, inherited: Option<String>) -> Option<String> {
        let Some((_, config)) = self.env_config
                                    .as_ref()
                                    .filter(|(variable, _)| *variable == name)
        else {
            return inherited;
        };
        let mut merged = match inherited.and_then(|text| serde_json::from_str(&text).ok()) {
            Some(Value::Object(object)) => object,
            _ => Map::new(),
        };
        config_env::merge_deep(&mut merged, config.clone());
        Some(Value::Object(merged).to_string())
    }
}

/// `options` split into words for `parse`, or all of it ignored when its
/// quoting is unbalanced.
fn with_words(options: &str, parse: fn(Vec<String>) -> AdapterOptions) -> AdapterOptions {
    match shell_words::split(options) {
        Ok(words) => parse(words),
        Err(_) => all_ignored(options),
    }
}

fn all_ignored(options: &str) -> AdapterOptions {
    AdapterOptions { ignored: vec![options.trim().to_owned()],
                     ..AdapterOptions::default() }
}

/// Claude's options: permission flags to a mode, the rest to `extraArgs`,
/// which the SDK writes back out as `--name value`, or `--name` for a
/// `null`. A repeated flag keeps its last value - `extraArgs` is a map.
fn claude_options(options: &str) -> AdapterOptions {
    let Ok(words) = shell_words::split(options)
    else {
        return all_ignored(options);
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
                     ignored,
                     ..AdapterOptions::default() }
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
