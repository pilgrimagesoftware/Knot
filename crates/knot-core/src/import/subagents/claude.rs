//! Claude Code's subagent definitions: `*.md` files whose YAML frontmatter
//! names them and whose body is the system prompt.
//!
//! Contract: `openspec/specs/data-import/spec.md` - "Claude subagent source
//! and format". The format here was read from a real `~/.claude/agents`
//! directory, not inferred.

use std::fs;
use std::path::{Path, PathBuf};

use directories::BaseDirs;

use super::{SubagentDefinition, SubagentProvider, SubagentScan};
use crate::consts::{
    CLAUDE_AGENTS_SUBPATH, FRONTMATTER_DELIMITER, SUBAGENT_DEFINITION_EXTENSION, SUBAGENT_NAME_KEY,
};
use crate::import::result::{Unreadable, UnreadableReason};

#[cfg(test)]
mod tests;

/// Reads `~/.claude/agents/*.md` and, for a folder, its `.claude/agents/*.md`.
pub struct ClaudeSubagentProvider;

impl SubagentProvider for ClaudeSubagentProvider {
    fn definitions(&self, folder: Option<&Path>) -> SubagentScan {
        let mut scan = SubagentScan::default();
        for dir in definition_dirs(folder) {
            read_dir_into(&dir, &mut scan);
        }
        scan
    }
}

/// The directories to read, user-level first so a project definition of the
/// same name is listed after the one it shadows rather than before it.
fn definition_dirs(folder: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(base) = BaseDirs::new() {
        dirs.push(base.home_dir().join(CLAUDE_AGENTS_SUBPATH));
    }
    if let Some(folder) = folder {
        dirs.push(folder.join(CLAUDE_AGENTS_SUBPATH));
    }
    dirs
}

/// Read every `.md` file in `dir` into `scan`. A missing or unreadable
/// directory contributes nothing and is not an error: the tool may not be
/// installed.
fn read_dir_into(dir: &Path, scan: &mut SubagentScan) {
    let Ok(entries) = fs::read_dir(dir)
    else {
        return;
    };
    let mut paths: Vec<PathBuf> =
        entries.flatten()
               .map(|entry| entry.path())
               .filter(|path| {
                   path.extension()
                       .is_some_and(|ext| ext == SUBAGENT_DEFINITION_EXTENSION)
               })
               .collect();
    // `read_dir` yields in filesystem order, which differs between machines;
    // sorting keeps the offered list stable so the same import twice shows the
    // same order.
    paths.sort();

    for path in paths {
        let label = file_label(&path);
        match fs::read_to_string(&path) {
            Ok(text) => match parse_definition(&text, &path) {
                Ok(definition) => scan.definitions.push(definition),
                Err(reason) => scan.unreadable.push(Unreadable::new(label, reason)),
            },
            Err(_) => {
                scan.unreadable
                    .push(Unreadable::new(label, UnreadableReason::Unreadable));
            }
        }
    }
}

/// How a file identifies itself in a result: its file name, falling back to
/// the whole path when there is none.
fn file_label(path: &Path) -> String {
    path.file_name().map_or_else(|| path.display().to_string(),
                                 |n| n.to_string_lossy().into_owned())
}

/// Split a definition into its frontmatter `name` and its body.
///
/// Keys Knot has no equivalent for - `description`, `color`, `model`, `tools` -
/// are dropped rather than folded into the instructions, per the contract.
fn parse_definition(text: &str, path: &Path) -> Result<SubagentDefinition, UnreadableReason> {
    let (frontmatter, body) = split_frontmatter(text).ok_or(UnreadableReason::NoFrontmatter)?;
    let name = frontmatter_name(frontmatter).ok_or(UnreadableReason::NoName)?;
    let instructions = body.trim();
    if instructions.is_empty() {
        return Err(UnreadableReason::EmptyBody);
    }
    Ok(SubagentDefinition { name,
                            instructions: instructions.to_string(),
                            source: path.display().to_string() })
}

/// Split `text` into the frontmatter block and everything after it.
///
/// The frontmatter is the region between a leading `---` line and the next
/// `---` line. `None` when the file does not open with the delimiter or never
/// closes it.
fn split_frontmatter(text: &str) -> Option<(&str, &str)> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let rest = text.trim_start_matches(['\r', '\n']);
    let after_open = rest.strip_prefix(FRONTMATTER_DELIMITER)?;
    // The opening delimiter must be a line of its own, not the start of a
    // longer token such as a `----` rule.
    let after_open = after_open.strip_prefix('\n')
                               .or_else(|| after_open.strip_prefix("\r\n"))?;

    let mut offset = 0;
    for line in after_open.split_inclusive('\n') {
        if line.trim_end() == FRONTMATTER_DELIMITER {
            return Some((&after_open[..offset], &after_open[offset + line.len()..]));
        }
        offset += line.len();
    }
    None
}

/// The frontmatter's `name` value.
///
/// Scanned rather than parsed as YAML: the one key Knot reads is a scalar at
/// the top level, and every other key is dropped unread, so a YAML dependency
/// would buy nothing. A top-level key starts at column zero, which is what
/// keeps this from matching a `name:` nested inside another key's value -
/// block scalars and nested mappings are indented by definition.
fn frontmatter_name(frontmatter: &str) -> Option<String> {
    frontmatter.lines()
               .filter_map(|line| line.strip_prefix(SUBAGENT_NAME_KEY))
               .map(str::trim)
               .map(unquote)
               .find(|name| !name.is_empty())
}

/// Strip one layer of matching quotes, so `name: "foo"` and `name: foo` agree.
fn unquote(value: &str) -> String {
    for quote in ['"', '\''] {
        if value.len() >= 2 && value.starts_with(quote) && value.ends_with(quote) {
            return value[1..value.len() - 1].to_string();
        }
    }
    value.to_string()
}
