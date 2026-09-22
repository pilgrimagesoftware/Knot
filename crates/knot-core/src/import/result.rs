//! What an import reports when it finishes: what it added, what it skipped as
//! already present, and what it could not read.
//!
//! Every import returns this same shape, so the UI renders one summary rather
//! than one per source. Unreadable records are carried by name and reason
//! because `data-import` requires a partial failure to be visible - a count
//! alone cannot tell the user *which* of seventeen files to go and fix.

use crate::l10n::{t, t_with};

#[cfg(test)]
mod tests;

/// Why a record a source held could not be turned into one of Knot's.
///
/// Each variant names a condition the reader checked for, so the reason
/// reaches the user as a sentence rather than as "failed". Not serialized,
/// hence no `FromStr`: it is produced by a reader and consumed by a renderer
/// in the same run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnreadableReason {
    /// The file has no `---`-delimited frontmatter block.
    NoFrontmatter,
    /// The frontmatter carries no `name` key. The file name is not used as a
    /// fallback: a guessed name is what `data-import` refuses.
    NoName,
    /// Nothing follows the frontmatter, so there are no instructions to
    /// import.
    EmptyBody,
    /// The file could not be read from disk at all.
    Unreadable,
    /// The source's own encoding could not be decoded - a malformed JSON blob
    /// inside Skwad's preferences, say.
    Malformed,
    /// An agent referenced a persona the source does not hold. The agent is
    /// still imported, without one; this names the reference that was lost.
    MissingPersona,
}

impl UnreadableReason {
    /// The catalogue key for this reason's copy. The copy itself lives in
    /// `locales/en.yml`, per the user-facing-text convention.
    #[must_use]
    pub fn l10n_key(self) -> &'static str {
        match self {
            Self::NoFrontmatter => "import.reason.no_frontmatter",
            Self::NoName => "import.reason.no_name",
            Self::EmptyBody => "import.reason.empty_body",
            Self::Unreadable => "import.reason.unreadable",
            Self::Malformed => "import.reason.malformed",
            Self::MissingPersona => "import.reason.missing_persona",
        }
    }
}

/// A record the source held that could not be understood, named so the user
/// can find it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreadable {
    /// How the record identifies itself to the user - a file name for a
    /// subagent definition, a record name or id for a Skwad record.
    pub name:   String,
    pub reason: UnreadableReason,
}

impl Unreadable {
    pub fn new(name: impl Into<String>, reason: UnreadableReason) -> Self {
        Self { name: name.into(),
               reason }
    }
}

/// The outcome of one import run.
///
/// The three lists are disjoint and together account for everything the
/// source offered. An empty result means the source had nothing, which is a
/// success - `data-import` requires a tool that is not installed to raise no
/// error.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ImportResult {
    /// Names of the records brought into Knot.
    pub added:      Vec<String>,
    /// Names of the records Knot already held, left untouched.
    pub skipped:    Vec<String>,
    /// Records the reader could not understand, each with its reason.
    pub unreadable: Vec<Unreadable>,
}

impl ImportResult {
    /// Whether the run touched nothing at all - nothing added, nothing
    /// skipped, nothing broken.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.skipped.is_empty() && self.unreadable.is_empty()
    }

    /// One line per non-empty category, ready to render.
    ///
    /// Each line carries both the count and the names: the count answers "did
    /// it do anything", the names answer "what", and an unreadable record's
    /// line also carries why. A category with nothing in it contributes no
    /// line rather than an empty one.
    #[must_use]
    pub fn summary_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        if !self.added.is_empty() {
            lines.push(count_and_names("import.result.added", &self.added));
        }
        if !self.skipped.is_empty() {
            lines.push(count_and_names("import.result.skipped", &self.skipped));
        }
        if !self.unreadable.is_empty() {
            let names: Vec<String> = self.unreadable
                                         .iter()
                                         .map(|u| {
                                             t_with("import.result.unreadable_entry",
                                                    &[("name", &u.name),
                                                      ("reason", &t(u.reason.l10n_key()))])
                                         })
                                         .collect();
            lines.push(count_and_names("import.result.unreadable", &names));
        }
        lines
    }
}

/// `key`'s sentence with the list's length and its entries substituted in.
fn count_and_names(key: &str, names: &[String]) -> String {
    t_with(key,
           &[("count", &names.len().to_string()),
             ("names", &names.join(", "))])
}
