//! Capability tags: the open vocabulary describing what an agent is for.
//!
//! Deliberately *not* a [`crate::settings::vocabulary`] enum. Those exist so
//! a closed set stops being a `String` matched with a `_ => default` arm.
//! This set is open on purpose - a new kind of agent (research,
//! infrastructure, review, testing, or one nobody has thought of) is added
//! by tagging it, not by adding a variant and recompiling. Nothing anywhere
//! matches on a tag, so the hazard that rule guards against cannot arise
//! here; see `openspec/specs/agent-registry/spec.md`.
//!
//! What a tag *does* need is one spelling. `Rust`, `rust ` and `RUST` are
//! the same capability, and an agent tagged one way must be found by a query
//! written the other. [`Capabilities`] therefore normalizes on construction,
//! on insertion and on deserialization, so the invariant holds by
//! construction and no caller has to remember it.

use std::collections::BTreeSet;
use std::collections::btree_set;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A set of normalized capability tags, ordered for stable output.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Capabilities(BTreeSet<String>);

/// Trims, lowercases, and rejects anything left empty.
fn normalize(tag: &str) -> Option<String> {
    let trimmed = tag.trim();
    if trimmed.is_empty() {
        None
    }
    else {
        Some(trimmed.to_lowercase())
    }
}

impl Capabilities {
    /// An agent with nothing declared about it.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a tag, normalized. Returns whether the set gained one - a tag
    /// that normalizes to empty, or to one already present, adds nothing.
    pub fn insert(&mut self, tag: &str) -> bool {
        normalize(tag).is_some_and(|tag| self.0.insert(tag))
    }

    /// Removes a tag, matching on its normalized form so a caller can pass
    /// whatever the user typed.
    pub fn remove(&mut self, tag: &str) -> bool {
        normalize(tag).is_some_and(|tag| self.0.remove(&tag))
    }

    /// Whether this set carries the tag, compared on the normalized form.
    pub fn contains(&self, tag: &str) -> bool {
        normalize(tag).is_some_and(|tag| self.0.contains(&tag))
    }

    /// Whether this set carries *every* tag in `required` - the matching
    /// rule a registry query uses.
    pub fn contains_all(&self, required: &Self) -> bool {
        required.0.is_subset(&self.0)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn iter(&self) -> btree_set::Iter<'_, String> {
        self.0.iter()
    }
}

impl<S: AsRef<str>> FromIterator<S> for Capabilities {
    fn from_iter<I: IntoIterator<Item = S>>(tags: I) -> Self {
        Self(tags.into_iter()
                 .filter_map(|tag| normalize(tag.as_ref()))
                 .collect())
    }
}

impl<'a> IntoIterator for &'a Capabilities {
    type IntoIter = btree_set::Iter<'a, String>;
    type Item = &'a String;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Serialize for Capabilities {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Capabilities {
    /// Normalizes on the way in, so a hand-edited settings file cannot
    /// introduce a tag that no query will ever match.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = Vec::<String>::deserialize(deserializer)?;
        Ok(raw.iter().collect())
    }
}

#[cfg(test)]
mod tests;
