//! Whether a proposed set of shortcuts may be installed.
//!
//! Pure, so every rejection rule is testable without a window.

use crate::keymap::Chord;
use crate::keymap::Resolved;
use crate::keymap::Shortcut;
use crate::keymap::chord::has_primary_modifier;
use crate::keymap::fixed::taken_chords;

/// Why a customization was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Rejection {
    /// The chord or family modifier holds none of ⌘, ⌃ and ⌥.
    NeedsModifier,
    /// The chord is already taken; carries what takes it, for the message.
    Conflict { chord: Chord, holder: String },
}

impl Rejection {
    pub(crate) fn message(&self) -> String {
        match self {
            Rejection::NeedsModifier => knot_core::l10n::t("keymap.rejection.needs_modifier"),
            Rejection::Conflict { chord, holder } => {
                knot_core::l10n::t_with("keymap.rejection.conflict",
                                        &[("chord", &chord.label()), ("holder", holder)])
            }
        }
    }
}

/// Checks `candidate` after `changed` was edited: that `changed` still has
/// a primary modifier, and that none of its chords is held by another
/// configurable shortcut, a fixed one or a reserved one.
///
/// Only `changed` is checked, because every change goes through here one
/// shortcut at a time, starting from a set that passed.
pub(crate) fn validate(candidate: &Resolved, changed: Shortcut) -> Result<(), Rejection> {
    let has_modifier = match candidate.modifiers(changed) {
        Some(modifiers) => has_primary_modifier(modifiers),
        None => candidate.chord(changed)
                         .is_some_and(Chord::has_primary_modifier),
    };
    if !has_modifier {
        return Err(Rejection::NeedsModifier);
    }

    let mine = candidate.chords_of(changed);
    let conflict = |chord: &Chord, holder: String| {
        Err(Rejection::Conflict { chord: chord.clone(),
                                  holder })
    };
    for other in Shortcut::ALL.into_iter().filter(|other| *other != changed) {
        if let Some(chord) = candidate.chords_of(other)
                                      .iter()
                                      .find(|chord| mine.contains(chord))
        {
            return conflict(chord, other.label());
        }
    }
    if let Some((chord, label_key)) = taken_chords().iter()
                                                    .find(|(chord, _)| mine.contains(chord))
    {
        return conflict(chord, knot_core::l10n::t(label_key));
    }
    Ok(())
}
