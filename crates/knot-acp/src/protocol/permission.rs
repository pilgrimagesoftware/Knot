//! `session/request_permission`: what the agent asks, the options it
//! offers, and which option a decision answers with
//! (<https://agentclientprotocol.com/protocol/tool-calls>, "Requesting
//! Permission").
//!
//! Each option carries a `kind` - allow or reject, once or always - and a
//! decision is resolved against those kinds, never against the options'
//! order or wording. Claude's adapter lists "Always Allow" first, so the
//! order-based rule this replaced answered every plain Allow as Always
//! Allow (#530).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// A `session/request_permission` request from the agent, awaiting the
/// caller's decision.
#[derive(Debug, Clone, PartialEq)]
pub struct PermissionRequest {
    pub rpc_id:          Value,
    pub tool_call_id:    String,
    pub tool_call_title: Option<String>,
    pub options:         Vec<PermissionOption>,
}

impl PermissionRequest {
    /// Whether `decision` names an option this request offers - an Always
    /// Allow the agent did not offer is not a decision the user can make.
    #[must_use]
    pub fn offers(&self, decision: PermissionDecision) -> bool {
        decision.option_id(&self.options).is_some()
    }

    /// Whether any option says what kind it is. Without kinds the options
    /// are read the way they were before kinds were parsed.
    #[must_use]
    pub fn has_kinds(&self) -> bool {
        has_kinds(&self.options)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PermissionOption {
    #[serde(rename = "optionId")]
    pub option_id: String,
    pub name:      String,
    /// What choosing this option means. `None` when the agent sent no kind,
    /// or one this client does not know - which leaves the option offered
    /// rather than failing the whole request.
    #[serde(default, deserialize_with = "known_kind")]
    pub kind:      Option<PermissionOptionKind>,
}

/// The four kinds an option can be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionOptionKind {
    AllowOnce,
    AllowAlways,
    RejectOnce,
    RejectAlways,
}

impl PermissionOptionKind {
    const ALL: [Self; 4] = [Self::AllowOnce,
                            Self::AllowAlways,
                            Self::RejectOnce,
                            Self::RejectAlways];

    /// The protocol's spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AllowOnce => "allow_once",
            Self::AllowAlways => "allow_always",
            Self::RejectOnce => "reject_once",
            Self::RejectAlways => "reject_always",
        }
    }
}

impl fmt::Display for PermissionOptionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An unknown kind is not one of the four.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownKind(pub String);

impl FromStr for PermissionOptionKind {
    type Err = UnknownKind;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL.into_iter()
                 .find(|kind| kind.as_str() == value)
                 .ok_or_else(|| UnknownKind(value.to_owned()))
    }
}

/// Reads `kind` leniently: an unknown or malformed value is `None`, so one
/// option from a newer protocol revision cannot empty the whole list.
fn known_kind<'de, D>(deserializer: D) -> Result<Option<PermissionOptionKind>, D::Error>
    where D: Deserializer<'de> {
    let raw = Option::<Value>::deserialize(deserializer)?;
    Ok(raw.as_ref()
          .and_then(Value::as_str)
          .and_then(|kind| kind.parse().ok()))
}

/// What the user decided about a permission request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionDecision {
    /// Allow this call only: the `allow_once` option.
    Allow,
    /// Allow it from now on: the `allow_always` option.
    AllowAlways,
    /// Refuse it: the `reject_once` option, else `reject_always` - refusing
    /// more than asked is the safe direction, allowing more is not.
    Deny,
    /// The option at this index, chosen by its own control.
    Choose(usize),
}

impl PermissionDecision {
    /// The index of the option this decision picks, when it picks one.
    ///
    /// With kinds, each decision takes its own kind and nothing else: an
    /// Allow on a request offering only Always Allow picks nothing, rather
    /// than granting more than the user chose. Without kinds - an adapter
    /// that predates them - Allow is the first option and Deny the first
    /// that reads as a refusal, as before.
    #[must_use]
    pub fn option_index(self, options: &[PermissionOption]) -> Option<usize> {
        let of_kind = |wanted: PermissionOptionKind| {
            options.iter()
                   .position(|option| option.kind == Some(wanted))
        };
        if let Self::Choose(index) = self {
            return (index < options.len()).then_some(index);
        }
        if !has_kinds(options) {
            return match self {
                Self::Allow => (!options.is_empty()).then_some(0),
                Self::Deny => {
                    options.iter().position(|option| {
                                      ["deny", "reject"].iter().any(|word| reads_as(option, word))
                                  })
                }
                Self::AllowAlways | Self::Choose(_) => None,
            };
        }
        match self {
            Self::Allow => of_kind(PermissionOptionKind::AllowOnce),
            Self::AllowAlways => of_kind(PermissionOptionKind::AllowAlways),
            Self::Deny => {
                of_kind(PermissionOptionKind::RejectOnce).or_else(|| {
                                                             of_kind(PermissionOptionKind::RejectAlways)
                                                         })
            }
            Self::Choose(_) => None,
        }
    }

    /// The option id to answer with, or `None` when this decision is not on
    /// offer.
    ///
    /// Without kinds, an Allow or Deny that matches no option still answers
    /// with the bare `allow` / `deny` it always did, so an adapter listing
    /// nothing recognisable is still answered rather than left hanging.
    #[must_use]
    pub fn option_id(self, options: &[PermissionOption]) -> Option<String> {
        if let Some(index) = self.option_index(options) {
            return Some(options[index].option_id.clone());
        }
        if has_kinds(options) {
            return None;
        }
        match self {
            Self::Allow => Some("allow".to_owned()),
            Self::Deny => Some("deny".to_owned()),
            Self::AllowAlways | Self::Choose(_) => None,
        }
    }
}

fn has_kinds(options: &[PermissionOption]) -> bool {
    options.iter().any(|option| option.kind.is_some())
}

fn reads_as(option: &PermissionOption, word: &str) -> bool {
    option.option_id.to_lowercase().contains(word) || option.name.to_lowercase().contains(word)
}

#[cfg(test)]
mod tests;
