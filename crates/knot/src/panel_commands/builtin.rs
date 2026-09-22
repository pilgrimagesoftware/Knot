//! Knot's own command vocabulary, as the panel lookup offers it.
//!
//! This list mirrors `plugin/claude/commands/*.md` - the command files knot
//! ships to the agents it runs. The two are kept in step by hand: a command
//! added to that directory is added here, or the lookup will not offer it.
//! The mirror exists because the plugin directory is not shipped inside the
//! app bundle's reach, and reading it per lookup would put a directory walk
//! on the render path for a set that only changes when knot itself does.

use super::LookupEntry;
use super::LookupSource;

/// `(token, description key)` per command, in the order the popup lists
/// them. Tokens carry no leading slash - the lookup adds it on insertion.
const BUILTIN_COMMANDS: &[(&str, &str)] = &[("broadcast", "panel.command.broadcast"),
                                            ("check", "panel.command.check"),
                                            ("create-agent", "panel.command.create_agent"),
                                            ("list-agents", "panel.command.list_agents"),
                                            ("list-repos", "panel.command.list_repos"),
                                            ("list-worktrees", "panel.command.list_worktrees"),
                                            ("send", "panel.command.send"),
                                            ("show-markdown", "panel.command.show_markdown"),
                                            ("worktree", "panel.command.worktree")];

/// The built-in half of the registry.
pub(super) struct BuiltinCommands;

impl LookupSource for BuiltinCommands {
    fn entries(&self) -> Vec<LookupEntry> {
        BUILTIN_COMMANDS.iter()
                        .map(|(token, key)| LookupEntry::new(*token, knot_core::l10n::t(key)))
                        .collect()
    }
}

#[cfg(test)]
pub(super) fn command_keys() -> impl Iterator<Item = &'static str> {
    BUILTIN_COMMANDS.iter().map(|(_, key)| *key)
}
