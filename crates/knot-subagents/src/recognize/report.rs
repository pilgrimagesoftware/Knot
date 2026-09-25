//! What a recognizer is handed about one tool call.
//!
//! Borrowed rather than owned, and `serde_json::Value` rather than a typed
//! ACP struct, so this crate does not depend on `knot-acp`. The fold already
//! holds the decoded update; a recognizer only needs to look at it, and the
//! two feeds would otherwise have to agree on a protocol type that only one
//! of them speaks.

use serde_json::Value;

/// One tool call, as much of it as recognition needs.
///
/// Every field but `id` is optional because ACP makes them optional: a
/// `tool_call_update` may carry only a status, and an adapter need not send a
/// metadata envelope at all.
#[derive(Debug, Clone, Copy, Default)]
pub struct ToolCallReport<'a> {
    /// The tool call's id, which becomes the subagent's identity.
    pub id:          &'a str,
    /// The `_meta` envelope, verbatim. Where adapters say what a call
    /// actually is, since ACP's own `kind` and `title` do not.
    pub meta:        Option<&'a Value>,
    /// The `rawInput` object, verbatim - the model's tool-use input.
    pub raw_input:   Option<&'a Value>,
    /// The call's status word, absent on an update that does not change it.
    pub status:      Option<&'a str>,
    /// The call's textual result, flattened by the caller.
    ///
    /// The failure reason on this path lives in the tool call's content
    /// rather than in a field of its own, so a recognizer that wants to
    /// report why a subagent failed has to be given it.
    pub result_text: Option<&'a str>,
}

impl<'a> ToolCallReport<'a> {
    pub fn new(id: &'a str) -> Self {
        Self { id,
               meta: None,
               raw_input: None,
               status: None,
               result_text: None }
    }

    /// Reads a string at `path` under `_meta`, e.g. `["claudeCode",
    /// "toolName"]`.
    pub fn meta_str(&self, path: &[&str]) -> Option<&'a str> {
        Self::dig(self.meta?, path)?.as_str()
    }

    /// Reads a boolean at `path` under `_meta`.
    pub fn meta_bool(&self, path: &[&str]) -> Option<bool> {
        Self::dig(self.meta?, path)?.as_bool()
    }

    /// Reads a string field of `rawInput`.
    pub fn input_str(&self, key: &str) -> Option<&'a str> {
        self.raw_input?.get(key)?.as_str()
    }

    fn dig<'v>(value: &'v Value, path: &[&str]) -> Option<&'v Value> {
        path.iter().try_fold(value, |current, key| current.get(key))
    }
}
