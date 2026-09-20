//! Activity-tracking presets: which terminal signals drive the state machine.

/// A set of terminal activity sources that drive status changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivityTracking(u8);

impl ActivityTracking {
    /// Set when terminal output is observed.
    pub const TERMINAL_OUTPUT: Self = Self(1 << 0);

    /// Set when a user keystroke is observed.
    pub const USER_INPUT: Self = Self(1 << 1);

    /// Set for a Panel-mode (ACP-managed) agent: status is driven directly
    /// by ACP session events, never by terminal output or keystrokes -
    /// mutually exclusive with `TERMINAL_OUTPUT`/`USER_INPUT` in practice
    /// (a Panel-mode agent tracks only this).
    pub const ACP_UPDATES: Self = Self(1 << 2);

    /// No sources tracked; the agent stays Idle (shell agents).
    pub const NONE: Self = Self(0);

    /// Both terminal sources tracked (the non-Panel-mode, non-shell default).
    pub const ALL: Self = Self(0b11);

    /// Whether `other` is a subset of this set.
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Whether no sources are tracked.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Add `other` to this set.
    pub const fn insert(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// Remove `other` from this set.
    pub const fn remove(self, other: Self) -> Self {
        Self(self.0 & !other.0)
    }
}

impl Default for ActivityTracking {
    fn default() -> Self {
        Self::ALL
    }
}

/// Tracking preset for `agent_type` in `view_mode`: shell agents track
/// nothing; a Panel-mode agent tracks only ACP updates (terminal output and
/// keystrokes are ignored even if its terminal still exists); every other
/// agent tracks both terminal sources.
pub fn tracking_for(agent_type: &str, view_mode: knot_core::ViewMode) -> ActivityTracking {
    if agent_type == "shell" {
        ActivityTracking::NONE
    }
    else if view_mode == knot_core::ViewMode::Panel {
        ActivityTracking::ACP_UPDATES
    }
    else {
        ActivityTracking::ALL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_tracks_nothing() {
        assert_eq!(tracking_for("shell", knot_core::ViewMode::Terminal),
                   ActivityTracking::NONE);
        assert!(tracking_for("shell", knot_core::ViewMode::Terminal).is_empty());
        // Shell agents never leave Idle regardless of view mode.
        assert_eq!(tracking_for("shell", knot_core::ViewMode::Panel),
                   ActivityTracking::NONE);
    }

    #[test]
    fn non_shell_tracks_both_in_terminal_mode() {
        for t in ["claude", "codex", "opencode", "gemini"] {
            assert_eq!(tracking_for(t, knot_core::ViewMode::Terminal),
                       ActivityTracking::ALL);
        }
    }

    #[test]
    fn unknown_agent_type_tracks_both_in_terminal_mode() {
        assert_eq!(tracking_for("unknown-type", knot_core::ViewMode::Terminal),
                   ActivityTracking::ALL);
    }

    #[test]
    fn panel_mode_tracks_only_acp_updates() {
        assert_eq!(tracking_for("claude", knot_core::ViewMode::Panel),
                   ActivityTracking::ACP_UPDATES);
        assert!(
            !tracking_for("claude", knot_core::ViewMode::Panel)
                .contains(ActivityTracking::TERMINAL_OUTPUT)
        );
        assert!(
            !tracking_for("claude", knot_core::ViewMode::Panel)
                .contains(ActivityTracking::USER_INPUT)
        );
    }

    #[test]
    fn contains_and_bits() {
        let user_only = ActivityTracking::ALL.remove(ActivityTracking::TERMINAL_OUTPUT);
        assert_eq!(user_only, ActivityTracking::USER_INPUT);
        assert!(!user_only.contains(ActivityTracking::TERMINAL_OUTPUT));
        assert!(user_only.contains(ActivityTracking::USER_INPUT));
        assert_eq!(user_only.insert(ActivityTracking::TERMINAL_OUTPUT),
                   ActivityTracking::ALL);
    }
}
