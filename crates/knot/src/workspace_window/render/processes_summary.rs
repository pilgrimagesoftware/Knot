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

use knot_processes::DescendantProcess;

#[cfg(test)]
mod tests;

/// How many distinct names the collapsed header lists before it summarizes
/// the rest as a remainder. Three fits the header beside the label at the
/// narrowest pane width without the row wrapping.
const MAX_NAMES: usize = 3;

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

/// The distinct process names, in the order the list already carries
/// (background first, longest-running first within each group).
///
/// Deduplicated: an agent running eight `node` workers should read `node`,
/// not `node, node, node`. The count of what was folded away is the
/// remainder's job, and it counts processes rather than names - "+5 more"
/// after `node` means five more processes, which is what a user glancing at
/// the header is asking.
fn distinct_names(processes: &[DescendantProcess]) -> Vec<&str> {
    let mut names: Vec<&str> = Vec::new();

    for process in processes {
        let name = process_name(&process.command);
        if !name.is_empty() && !names.contains(&name) {
            names.push(name);
        }
    }

    names
}

/// The collapsed header's list: up to [`MAX_NAMES`] names, then a remainder
/// counting the processes those names do not cover.
fn names_text(processes: &[DescendantProcess]) -> String {
    let names = distinct_names(processes);
    let shown: Vec<&str> = names.iter().take(MAX_NAMES).copied().collect();
    let listed = shown.join(&separator());

    let covered = processes.iter()
                           .filter(|process| shown.contains(&process_name(&process.command)))
                           .count();
    let remainder = processes.len() - covered;

    if remainder == 0 {
        return listed;
    }

    knot_core::l10n::t_with("processes.summary_more",
                            &[("names", &listed), ("count", &remainder.to_string())])
}

/// What goes between two names in the list, from the catalog: the comma and
/// the space after it are punctuation, and punctuation is the translator's.
fn separator() -> String {
    knot_core::l10n::t("processes.summary_separator")
}

/// The header's summary, for an agent whose section is on screen.
///
/// `is_running` comes first because it settles the question before the
/// snapshot is consulted: an agent that has stopped has nothing running
/// whatever its last sample said, and leaving the stale names up would be the
/// header contradicting the body's "not running".
pub(super) fn summary_text(is_running: bool, expanded: bool,
                           processes: Option<&[DescendantProcess]>)
                           -> String {
    if !is_running {
        return knot_core::l10n::t("processes.count_none");
    }

    match processes {
        // Genuinely unknown: the section is shown and its first sample has
        // not landed. Seconds, now that being shown is what starts the
        // sampler - not forever, as it was when expansion was.
        None => knot_core::l10n::t("processes.count_unknown"),
        Some([]) => knot_core::l10n::t("processes.count_none"),
        // Expanded, the rows are the detail; the header counts them.
        Some(list) if expanded => knot_core::l10n::pluralize(list.len() as u64,
                                                             "processes.count_one",
                                                             "processes.count_many"),
        Some(list) => names_text(list),
    }
}
