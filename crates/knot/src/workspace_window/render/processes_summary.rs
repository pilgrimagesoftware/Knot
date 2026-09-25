//! What the processes header says beside its label.
//!
//! Contract: `openspec/specs/agent-processes/spec.md` - "The agent's pane
//! presents a processes section".
//!
//! The header used to read "Counting…" forever. It showed a count drawn from
//! the last sample, but sampling only ran while the section was *expanded*,
//! and the section starts collapsed - so the count it promised could never
//! arrive. Collapsing again called `clear()`, which put it back to "Counting…"
//! even after an expansion had filled it in.
//!
//! Collapsed now names what is running, which is the thing worth knowing
//! without opening anything; expanded counts, because the rows below are
//! already the detail. Both need a sample while collapsed, which is why the
//! sampler's gate is "shown" rather than "expanded".
//!
//! Subagents come first in both states. A delegation is the fact a user
//! cannot get anywhere else in Knot - a `node` under an agent is visible from
//! Activity Monitor, and "waiting on three subagents" is not.

use knot_processes::DescendantProcess;
use knot_subagents::Subagent;

#[cfg(test)]
mod tests;

/// How many distinct names the collapsed header lists before it summarizes
/// the rest as a remainder. Three fits the header beside the label at the
/// narrowest pane width without the row wrapping.
const MAX_NAMES: usize = 3;

/// Everything the header needs, grouped rather than passed as four
/// positional arguments - three of them are `Option`s or `bool`s, and at a
/// call site those are indistinguishable from each other.
#[derive(Debug, Clone, Copy)]
pub(super) struct Summary<'a> {
    pub(super) is_running: bool,
    pub(super) expanded:   bool,
    /// The last completed sample, or `None` if none has landed - which the
    /// header renders as unknown rather than as zero.
    pub(super) processes:  Option<&'a [DescendantProcess]>,
    /// This agent's subagents, or `None` when its type cannot report them.
    ///
    /// `None` and `Some(&[])` are different answers and must stay so: the
    /// first means the subagents group is not drawn at all, so the header
    /// must not claim a count of zero for something the user cannot see.
    pub(super) subagents:  Option<&'a [Subagent]>,
}

/// A process's short name: the leading word of its command line, less any
/// directories.
///
/// The command is the full line (`/opt/homebrew/bin/node --inspect server.js`)
/// and the header has room for `node`. Arguments are dropped rather than
/// truncated, because the first thing to fall off the end of a long path is
/// the part that identifies the program.
pub(super) fn process_name(command: &str) -> &str {
    let first_word = command.split_whitespace().next().unwrap_or("");

    first_word.rsplit('/')
              .next()
              .filter(|name| !name.is_empty())
              .unwrap_or(first_word)
}

/// A subagent's short name: the kind the agent named, or the catalogue's word
/// for one that named none.
///
/// Owned rather than borrowed because the absent case comes from the
/// catalogue, not from the record.
fn subagent_name(subagent: &Subagent) -> String {
    subagent.kind
            .name()
            .map_or_else(|| knot_core::l10n::t("processes.subagent_kind_unstated"),
                         str::to_owned)
}

/// The distinct names, subagent kinds first, then process names.
///
/// Deduplicated across both: an agent running eight `node` workers should
/// read `node`, not `node, node, node`, and the same holds for three
/// `code-review` subagents. Deduping across the two kinds as well is
/// deliberate - a process and a subagent sharing a name is vanishingly rare,
/// and listing the word twice would read as a bug rather than as precision.
fn distinct_names(summary: &Summary<'_>) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let mut push = |name: String| {
        if !name.is_empty() && !names.contains(&name) {
            names.push(name);
        }
    };

    for subagent in summary.subagents.unwrap_or_default() {
        push(subagent_name(subagent));
    }
    for process in summary.processes.unwrap_or_default() {
        push(process_name(&process.command).to_owned());
    }

    names
}

/// The collapsed header's list: up to [`MAX_NAMES`] names, then a remainder
/// counting what those names do not cover.
fn names_text(summary: &Summary<'_>) -> String {
    let names = distinct_names(summary);
    let shown: Vec<String> = names.iter().take(MAX_NAMES).cloned().collect();
    let listed = shown.join(&separator());

    let covered = summary.subagents
                         .unwrap_or_default()
                         .iter()
                         .filter(|subagent| shown.contains(&subagent_name(subagent)))
                         .count()
                  + summary.processes
                           .unwrap_or_default()
                           .iter()
                           .filter(|process| {
                               shown.iter()
                                    .any(|name| name == process_name(&process.command))
                           })
                           .count();
    let remainder = total(summary) - covered;

    if remainder == 0 {
        return listed;
    }

    knot_core::l10n::t_with("processes.summary_more",
                            &[("names", &listed), ("count", &remainder.to_string())])
}

/// How many things are running, of both kinds.
fn total(summary: &Summary<'_>) -> usize {
    summary.subagents.unwrap_or_default().len() + summary.processes.unwrap_or_default().len()
}

/// The expanded header's counts, each group counted separately.
///
/// Two subagents and three processes is not five of anything, so they are
/// never added together. A group with nothing in it is left out rather than
/// counted as zero - the group below already says so in words, and "0
/// subagents, 3 processes" spends the header's width on the half that has
/// nothing to report.
fn counts_text(summary: &Summary<'_>) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Some(subagents) = summary.subagents.filter(|list| !list.is_empty()) {
        parts.push(knot_core::l10n::pluralize(subagents.len() as u64,
                                              "processes.subagent_count_one",
                                              "processes.subagent_count_many"));
    }
    if let Some(processes) = summary.processes.filter(|list| !list.is_empty()) {
        parts.push(knot_core::l10n::pluralize(processes.len() as u64,
                                              "processes.count_one",
                                              "processes.count_many"));
    }

    parts.join(&separator())
}

/// What goes between two names in the list, from the catalog: the comma and
/// the space after it are punctuation, and punctuation is the translator's.
fn separator() -> String {
    knot_core::l10n::t("processes.summary_separator")
}

/// The header's summary, for an agent whose section is on screen.
///
/// `is_running` settles the question before anything else is consulted: an
/// agent that has stopped has nothing running whatever its last sample or its
/// last records said, and leaving either up would be the header contradicting
/// the body's "not running".
pub(super) fn summary_text(summary: &Summary<'_>) -> String {
    if !summary.is_running {
        return knot_core::l10n::t("processes.count_none");
    }

    // Genuinely unknown: the section is shown and its first *process* sample
    // has not landed. Subagents are never unknown - they are not sampled, so
    // there is no first sample to wait on - which is why a held subagent does
    // not rescue the header from this branch. Seconds, now that being shown
    // is what starts the sampler, not forever as it was when expansion was.
    if summary.processes.is_none() {
        return knot_core::l10n::t("processes.count_unknown");
    }

    if total(summary) == 0 {
        return knot_core::l10n::t("processes.count_none");
    }

    if summary.expanded {
        return counts_text(summary);
    }

    names_text(summary)
}
