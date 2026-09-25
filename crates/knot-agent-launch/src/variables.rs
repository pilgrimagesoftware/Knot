//! Prompt variables: the closed set of `{{name}}` placeholders a library
//! prompt or a custom startup prompt may use, and the one scanner that both
//! expands them and reports the names it does not know.
//!
//! Contract: `openspec/specs/prompt-library/spec.md`, "Prompt variables" and
//! "Unknown variables and escaping".
//!
//! [`expand`] is pure over a [`PromptContext`] whose values are already
//! resolved. Reading the branch runs `git`, so building the context is the
//! caller's job - see [`crate::startup`] - and each caller knows which thread
//! it is on. Keeping the scan pure is what keeps that read off the render
//! path.

use std::fmt;
use std::str::FromStr;

/// Opens a variable.
const OPEN: &str = "{{";
/// Closes a variable.
const CLOSE: &str = "}}";
/// Written before [`OPEN`] to mean a literal `{{`.
const ESCAPE: char = '\\';

/// One of the names that expand. Closed: anything else between the braces is
/// left exactly as typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PromptVariable {
    AgentName,
    AgentId,
    AgentType,
    Folder,
    FolderName,
    Workspace,
    Branch,
    Date,
}

impl PromptVariable {
    /// Every variable, in the order the editors list them.
    pub const ALL: [PromptVariable; 8] = [PromptVariable::AgentName,
                                          PromptVariable::AgentId,
                                          PromptVariable::AgentType,
                                          PromptVariable::Folder,
                                          PromptVariable::FolderName,
                                          PromptVariable::Workspace,
                                          PromptVariable::Branch,
                                          PromptVariable::Date];

    /// The name as written between the braces.
    pub fn name(self) -> &'static str {
        match self {
            Self::AgentName => "agent.name",
            Self::AgentId => "agent.id",
            Self::AgentType => "agent.type",
            Self::Folder => "folder",
            Self::FolderName => "folder.name",
            Self::Workspace => "workspace",
            Self::Branch => "branch",
            Self::Date => "date",
        }
    }

    /// The variable written the way a prompt uses it, `{{name}}`.
    pub fn placeholder(self) -> String {
        format!("{OPEN}{}{CLOSE}", self.name())
    }
}

impl fmt::Display for PromptVariable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// A name between the braces that is not a [`PromptVariable`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownVariable(pub String);

impl FromStr for PromptVariable {
    type Err = UnknownVariable;

    /// Case-sensitive: `Folder` is not `folder`.
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        Self::ALL.into_iter()
                 .find(|variable| variable.name() == name)
                 .ok_or_else(|| UnknownVariable(name.to_string()))
    }
}

/// Every value a prompt can be expanded with, already resolved.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PromptContext {
    pub agent_name: String,
    pub agent_id:   String,
    pub agent_type: String,
    /// Absolute path of the agent's folder.
    pub folder:     String,
    /// Empty when the agent belongs to no workspace.
    pub workspace:  String,
    /// Empty when the folder is not in a git repository.
    pub branch:     String,
    /// `YYYY-MM-DD`, local time.
    pub date:       String,
}

impl PromptContext {
    fn value(&self, variable: PromptVariable) -> String {
        match variable {
            PromptVariable::AgentName => self.agent_name.clone(),
            PromptVariable::AgentId => self.agent_id.clone(),
            PromptVariable::AgentType => self.agent_type.clone(),
            PromptVariable::Folder => self.folder.clone(),
            PromptVariable::FolderName => knot_core::folder_name(&self.folder).unwrap_or_default(),
            PromptVariable::Workspace => self.workspace.clone(),
            PromptVariable::Branch => self.branch.clone(),
            PromptVariable::Date => self.date.clone(),
        }
    }
}

/// `text` with every known variable replaced by its value.
///
/// A single left-to-right pass: values are appended to the output and never
/// rescanned, so a value holding `{{date}}` stays literal by construction.
pub fn expand(text: &str, context: &PromptContext) -> String {
    let mut out = String::with_capacity(text.len());
    scan(text, |piece| match piece {
        Piece::Text(text) => out.push_str(text),
        Piece::Variable(variable) => out.push_str(&context.value(variable)),
    });
    out
}

/// The names in `text` that look like variables but are not one, each once,
/// in the order they first appear - what the editors warn about.
pub fn unknown_variables(text: &str) -> Vec<String> {
    let mut unknown: Vec<String> = Vec::new();
    scan_names(text, |name| {
        if name.parse::<PromptVariable>().is_err() && !unknown.iter().any(|seen| seen == name) {
            unknown.push(name.to_string());
        }
    });
    unknown
}

enum Piece<'a> {
    Text(&'a str),
    Variable(PromptVariable),
}

/// Walk `text`, handing each literal run and each known variable to `emit`.
fn scan<'a>(text: &'a str, mut emit: impl FnMut(Piece<'a>)) {
    let mut rest = text;
    while let Some(open) = rest.find(OPEN) {
        let (before, from_open) = rest.split_at(open);
        if before.ends_with(ESCAPE) {
            emit(Piece::Text(&before[..before.len() - ESCAPE.len_utf8()]));
            emit(Piece::Text(OPEN));
            rest = &from_open[OPEN.len()..];
            continue;
        }
        emit(Piece::Text(before));
        let inner = &from_open[OPEN.len()..];
        let Some(close) = inner.find(CLOSE)
        else {
            // Unclosed: the rest is literal.
            emit(Piece::Text(from_open));
            return;
        };
        let whole = &from_open[..OPEN.len() + close + CLOSE.len()];
        match inner[..close].trim().parse::<PromptVariable>() {
            Ok(variable) => emit(Piece::Variable(variable)),
            Err(_) => emit(Piece::Text(whole)),
        }
        rest = &from_open[whole.len()..];
    }
    emit(Piece::Text(rest));
}

/// Walk `text`, handing every trimmed name found between unescaped braces to
/// `found`, known or not. Shares [`scan`]'s escaping and unclosed-brace rules.
fn scan_names<'a>(text: &'a str, mut found: impl FnMut(&'a str)) {
    let mut rest = text;
    while let Some(open) = rest.find(OPEN) {
        let (before, from_open) = rest.split_at(open);
        let inner = &from_open[OPEN.len()..];
        if before.ends_with(ESCAPE) {
            rest = inner;
            continue;
        }
        let Some(close) = inner.find(CLOSE)
        else {
            return;
        };
        found(inner[..close].trim());
        rest = &inner[close + CLOSE.len()..];
    }
}

#[cfg(test)]
mod tests;
