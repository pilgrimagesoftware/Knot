//! The entries one lookup draws on, collected once per focused agent.

use std::path::Path;

use crate::panel_commands::builtin;
use crate::panel_commands::entry::LookupEntry;
use crate::panel_commands::entry::LookupMatch;
use crate::panel_commands::entry::LookupSource;
use crate::panel_commands::entry::Matcher;
use crate::panel_commands::skills;

/// The entries one lookup draws on.
///
/// Collection reads the filesystem (see [`skills`]), so a registry is
/// built off the render path and cached - never rebuilt per keystroke.
#[derive(Clone, Debug)]
pub(crate) struct LookupRegistry {
    entries: Vec<LookupEntry>,
    matcher: Matcher,
}

impl Default for LookupRegistry {
    fn default() -> Self {
        Self { entries: Vec::new(),
               matcher: Matcher::Substring, }
    }
}

impl LookupRegistry {
    /// Collects every source in order, dropping an entry whose token an
    /// earlier source already claimed - so a skill cannot shadow a
    /// built-in command.
    ///
    /// The registry takes its matcher from the first source. A registry's
    /// sources are alternative places the *same kind* of thing comes from,
    /// so they share one; two kinds of thing are two registries, which is
    /// what commands and files are.
    pub(crate) fn from_sources(sources: &[&dyn LookupSource]) -> Self {
        let matcher = sources.first()
                             .map_or(Matcher::Substring, |source| source.matcher());
        let mut entries: Vec<LookupEntry> = Vec::new();
        for source in sources {
            for entry in source.entries() {
                if entries.iter().any(|seen| seen.token == entry.token) {
                    continue;
                }
                entries.push(entry);
            }
        }
        Self { entries, matcher }
    }

    /// The registry for the agent whose folder is `folder`: knot's
    /// built-in commands plus whatever skills that agent's roots hold.
    pub(crate) fn for_agent(folder: Option<&Path>) -> Self {
        let builtin = builtin::BuiltinCommands;
        let skills = skills::SkillRoots::for_agent(folder);
        Self::from_sources(&[&builtin, &skills])
    }

    /// The entries `filter` selects, in the matcher's order.
    pub(crate) fn matching(&self, filter: &str) -> Vec<LookupMatch> {
        self.matcher.matching(&self.entries, filter)
    }
}
