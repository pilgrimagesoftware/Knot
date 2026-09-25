//! The prompt library's record and the startup prompt an agent or bench entry
//! carries.
//!
//! Contract: `openspec/specs/prompt-library/spec.md`.
//!
//! Only the stored shapes live here. Expanding a prompt's variables and
//! resolving a [`StartupPrompt`] against the library are launch concerns and
//! live in `knot-agent-launch`, next to the initialization prompt they follow.

use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

/// One named, reusable prompt in the library.
///
/// Built through [`Prompt::new`], which refuses a blank name or text, so a
/// stored prompt always has something to show and something to send. Names
/// need not be unique; the id is what an agent's startup prompt refers to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    pub id:   Uuid,
    pub name: String,
    /// Stored exactly as entered, line breaks and variables included; the
    /// variables are expanded when the prompt is used, never here.
    pub text: String,
}

impl Prompt {
    /// A new prompt with a fresh id, or `None` when the name or text is blank
    /// after trimming.
    pub fn new(name: impl Into<String>, text: impl Into<String>) -> Option<Self> {
        let (name, text) = (name.into(), text.into());
        if is_blank(&name) || is_blank(&text) {
            return None;
        }
        Some(Self { id: Uuid::new_v4(),
                    name,
                    text })
    }
}

/// The prompt sent as its own turn after the initialization prompt, on a
/// fresh session.
///
/// "No startup prompt" is the absence of one - `Option<StartupPrompt>` on the
/// record - rather than a third variant, so a record written before the field
/// existed decodes to it with no migration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum StartupPrompt {
    /// A library prompt, by id. Kept when the prompt is removed, so an editor
    /// can show the reference as missing; it then resolves to nothing.
    Library(Uuid),
    /// Text held on the agent or bench entry itself.
    Custom(String),
}

impl StartupPrompt {
    /// Custom text as a startup prompt, or `None` when it is blank: blank
    /// custom text is stored as no startup prompt at all.
    pub fn custom(text: impl Into<String>) -> Option<Self> {
        let text = text.into();
        (!is_blank(&text)).then_some(Self::Custom(text))
    }
}

/// Serde reader for an `Option<StartupPrompt>` field that applies
/// [`StartupPrompt::custom`]'s blank rule to what is on disk, so a
/// hand-edited or older document holding blank custom text reads as none.
pub(crate) fn de_startup_prompt<'de, D>(deserializer: D) -> Result<Option<StartupPrompt>, D::Error>
    where D: Deserializer<'de> {
    Ok(match Option::<StartupPrompt>::deserialize(deserializer)? {
        Some(StartupPrompt::Custom(text)) => StartupPrompt::custom(text),
        other => other,
    })
}

fn is_blank(text: &str) -> bool {
    text.trim().is_empty()
}

#[cfg(test)]
mod tests;
