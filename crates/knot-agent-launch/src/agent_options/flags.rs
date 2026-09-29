//! Splits option words against a table of the flags an adapter accepts.
//!
//! Claude's parser can take any `--name`, because `extraArgs` hands every
//! one to a CLI that validates it. The other adapters get their options
//! through a config object or a command line that fails the launch on a word
//! it does not expect, and a boolean flag followed by a stray word would
//! pass that word as the agent's first prompt. So each one names the flags
//! it takes and whether each takes a value, and nothing else goes through.

/// One flag an adapter takes: its long name, its one-letter alias if any,
/// and whether a value follows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FlagSpec {
    pub long:        &'static str,
    pub short:       Option<char>,
    pub takes_value: bool,
}

impl FlagSpec {
    pub(crate) const fn value(long: &'static str, short: Option<char>) -> Self {
        Self { long,
               short,
               takes_value: true }
    }

    pub(crate) const fn switch(long: &'static str, short: Option<char>) -> Self {
        Self { long,
               short,
               takes_value: false }
    }
}

/// A flag from the table, with the value the user gave it. A switch's value
/// is what followed `=`, if the user wrote one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Given {
    pub spec:  FlagSpec,
    pub value: Option<String>,
}

impl Given {
    /// The flag as command-line words, under its long name.
    pub(crate) fn words(&self) -> Vec<String> {
        match (&self.value, self.spec.takes_value) {
            (Some(value), true) => vec![format!("--{}", self.spec.long), value.clone()],
            (Some(value), false) => vec![format!("--{}={value}", self.spec.long)],
            (None, _) => vec![format!("--{}", self.spec.long)],
        }
    }
}

/// `words` split into the flags `table` names and the words it does not.
///
/// A flag not in the table is ignored together with the word after it when
/// that word is not itself a flag, since it was most likely the unknown
/// flag's value and would otherwise pass as a stray word. A value flag with
/// no value is ignored too.
pub(crate) fn split(words: Vec<String>, table: &[FlagSpec]) -> (Vec<Given>, Vec<String>) {
    let mut given = Vec::new();
    let mut ignored = Vec::new();
    let mut words = words.into_iter().peekable();
    while let Some(word) = words.next() {
        let Some((name, inline)) = flag_name(&word)
        else {
            ignored.push(word);
            continue;
        };
        let spec = table.iter().find(|spec| match name {
                                   Name::Long(long) => spec.long == long,
                                   Name::Short(short) => spec.short == Some(short),
                               });
        let Some(&spec) = spec
        else {
            ignored.push(word);
            if inline.is_none()
               && let Some(value) = words.next_if(|next| !next.starts_with('-'))
            {
                ignored.push(value);
            }
            continue;
        };
        let value = if spec.takes_value {
            inline.or_else(|| words.next_if(|next| !next.starts_with('-')))
        }
        else {
            inline
        };
        if spec.takes_value && value.is_none() {
            ignored.push(word);
            continue;
        }
        given.push(Given { spec, value });
    }
    (given, ignored)
}

enum Name<'a> {
    Long(&'a str),
    Short(char),
}

/// A flag word's name and any `=value` written into it, or `None` for a
/// word that is not a flag.
fn flag_name(word: &str) -> Option<(Name<'_>, Option<String>)> {
    if let Some(flag) = word.strip_prefix("--").filter(|flag| !flag.is_empty()) {
        return Some(match flag.split_once('=') {
                        Some((name, value)) => (Name::Long(name), Some(value.to_owned())),
                        None => (Name::Long(flag), None),
                    });
    }
    let mut letters = word.strip_prefix('-')?.chars();
    let short = letters.next()?;
    letters.next()
           .is_none()
           .then_some((Name::Short(short), None))
}
