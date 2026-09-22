//! Closed vocabularies stored in [`crate::Settings`].
//!
//! Each was a `String` matched in several places with a `_ => default` arm,
//! so a corrupt or misspelled value rendered identically to the real default
//! and no amount of reading the code would tell you which you had. As enums
//! every match is exhaustive, the compiler finds the next site when a variant
//! is added, and an unrecognized stored value is turned into the default
//! *once*, at load, where it can be seen.
//!
//! The wire format is unchanged: each serializes to the same lowercase string
//! it has always been stored as, so no settings migration is needed.
//!
//! Deserialization never fails. `Settings` is one document, and a hard error
//! on one bad field would take the whole file - every agent, workspace and
//! persona - down with it. Unknown values degrade to the default instead.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// An unrecognized stored value, carrying what was found so a caller can say
/// so rather than silently substituting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownVariant(pub String);

impl fmt::Display for UnknownVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unrecognized value {:?}", self.0)
    }
}

macro_rules! settings_vocabulary {
    (
        $(#[$meta:meta])*
        $name:ident {
            $(#[$default_meta:meta])? $default_variant:ident = $default_str:literal,
            $($variant:ident = $str:literal),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
        #[serde(from = "String", into = "String")]
        pub enum $name {
            #[default]
            $default_variant,
            $($variant),*
        }

        impl $name {
            /// Every variant, in the order the settings picker offers them.
            pub const ALL: &'static [Self] = &[Self::$default_variant, $(Self::$variant),*];

            /// The string this variant is stored and transmitted as.
            pub const fn as_str(self) -> &'static str {
                match self {
                    Self::$default_variant => $default_str,
                    $(Self::$variant => $str),*
                }
            }

            /// Parses a stored value, falling back to the default and
            /// reporting what it could not read.
            ///
            /// Use this at the edge - reading settings, decoding a request -
            /// so an unrecognized value is noted once rather than quietly
            /// behaving as the default forever.
            pub fn from_stored(value: &str) -> (Self, Option<UnknownVariant>) {
                match value.parse() {
                    Ok(parsed) => (parsed, None),
                    Err(unknown) => (Self::default(), Some(unknown)),
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = UnknownVariant;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $default_str => Ok(Self::$default_variant),
                    $($str => Ok(Self::$variant),)*
                    other => Err(UnknownVariant(other.to_string())),
                }
            }
        }

        impl From<String> for $name {
            /// Tolerant by design - see the module docs on why one bad field
            /// must not fail the whole settings document.
            fn from(value: String) -> Self {
                Self::from_stored(&value).0
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.as_str().to_string()
            }
        }
    };
}

settings_vocabulary! {
    /// Which appearance the app paints in.
    ///
    /// `Auto` and `System` are both present because the Swift reference
    /// offered both and the stored values are in users' settings files.
    AppearanceMode {
        Auto = "auto",
        System = "system",
        Light = "light",
        Dark = "dark",
    }
}

settings_vocabulary! {
    /// Which LLM provider Autopilot asks when it needs a reply composed.
    AiProvider {
        OpenAi = "openai",
        Anthropic = "anthropic",
        Google = "google",
    }
}

settings_vocabulary! {
    /// What Autopilot does when an agent asks for input.
    AutopilotAction {
        Mark = "mark",
        Ask = "ask",
        Continue = "continue",
        Custom = "custom",
    }
}

/// How expensive an agent is to run, as declared by whoever configured it.
///
/// Deliberately hand-written rather than a [`settings_vocabulary!`] enum:
/// that macro pins the default to the first variant, and cost tier needs
/// both a *middle* default and a declaration order that carries the
/// ordering. Deriving `Ord` here is what lets the registry rank candidates
/// cheapest-first without a lookup table of magic numbers.
///
/// This is a declared class, not a measurement. Nothing in the app meters
/// tokens or spend; see `openspec/specs/agent-registry/spec.md`.
#[derive(Debug,
           Clone,
           Copy,
           PartialEq,
           Eq,
           PartialOrd,
           Ord,
           Hash,
           Default,
           Serialize,
           Deserialize)]
#[serde(from = "String", into = "String")]
pub enum CostTier {
    Low,
    #[default]
    Medium,
    High,
}

impl CostTier {
    /// Every variant, cheapest first - the order the registry ranks in and
    /// the order a picker offers them.
    pub const ALL: &'static [Self] = &[Self::Low, Self::Medium, Self::High];

    /// The string this variant is stored and transmitted as.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    /// Parses a stored value, falling back to the default and reporting what
    /// it could not read. Same contract as the macro-generated vocabularies:
    /// see the module docs on why one bad field must not fail the document.
    pub fn from_stored(value: &str) -> (Self, Option<UnknownVariant>) {
        match value.parse() {
            Ok(parsed) => (parsed, None),
            Err(unknown) => (Self::default(), Some(unknown)),
        }
    }
}

impl fmt::Display for CostTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CostTier {
    type Err = UnknownVariant;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            other => Err(UnknownVariant(other.to_string())),
        }
    }
}

impl From<String> for CostTier {
    fn from(value: String) -> Self {
        Self::from_stored(&value).0
    }
}

impl From<CostTier> for String {
    fn from(value: CostTier) -> Self {
        value.as_str().to_string()
    }
}

#[cfg(test)]
mod tests;
