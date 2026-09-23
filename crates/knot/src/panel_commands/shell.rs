//! Recognising the `!` that makes a prompt a shell command.
//!
//! Contract: `openspec/specs/panel-shell-passthrough/spec.md`.
//!
//! Unlike [`super::token`], this is not about the caret. A `/` token is
//! completed in place and can sit anywhere the caret is; a `!` consumes the
//! whole buffer, so the only thing that matters is what the buffer starts
//! with. Recognising it per line instead would make a multi-line prompt
//! containing an exclamation-led line silently executable.
//!
//! This decides *what* the buffer is, and nothing else: running it, drawing it
//! and telling an agent about it all live in `workspace_window::panel`.

/// The command in `value`, if the buffer is a shell command.
///
/// The test is on the raw buffer -- byte 0 must be `!` -- and deliberately not
/// on a trimmed one. That is what makes a single leading space the escape for
/// a prompt that has to begin with a literal `!`: the space is trimmed on the
/// way to the agent, so nothing the user typed is rewritten to achieve it.
///
/// `!` with nothing but whitespace after it is neither a command nor a
/// message, and yields `None`.
pub(crate) fn shell_command(value: &str) -> Option<&str> {
    let rest = value.strip_prefix('!')?;
    let command = rest.trim();

    (!command.is_empty()).then_some(command)
}

/// Whether `value` would run as a shell command.
///
/// Separate from [`shell_command`] because the input area asks this per frame,
/// to mark the line and to decide whether send is enabled, and reads nothing
/// out of the answer.
pub(crate) fn is_shell_command(value: &str) -> bool {
    shell_command(value).is_some()
}

/// Whether the send control is enabled, per `acp-panel-ui`'s "Input area send
/// control".
///
/// A pure function rather than an expression in the render, because the whole
/// requirement is which combinations of three flags it says yes to, and that
/// is worth asserting rather than reading.
///
/// A shell command is exempt from every reason a prompt may not be sent: it
/// runs locally and touches neither the session nor the queue, so a pending
/// permission and a turn in flight are both beside the point.
///
/// A buffer that opens with `!` and carries no command is refused outright
/// rather than falling through to the ordinary rules. Falling through is what
/// it did first, and the ordinary rules said yes -- `"!"` is not empty -- so
/// pressing send delivered a lone `!` to the agent as a prompt, which is the
/// one thing the spec says a bare bang must never be.
pub(crate) fn can_send(value: &str, blocked: bool, turn_active: bool) -> bool {
    if has_shell_trigger(value) {
        return is_shell_command(value);
    }

    !blocked && !turn_active && !value.trim().is_empty()
}

/// Whether the buffer opens with the trigger, whether or not a command
/// follows.
///
/// The distinction from [`is_shell_command`] is the whole of the bare-bang
/// rule: this is true for `!`, that is not, and a buffer where they disagree
/// must be submitted as neither a command nor a message.
pub(crate) fn has_shell_trigger(value: &str) -> bool {
    value.starts_with('!')
}

#[cfg(test)]
mod tests {
    use super::{is_shell_command, shell_command};

    #[test]
    fn a_leading_bang_is_a_command() {
        assert_eq!(shell_command("!ls -la"), Some("ls -la"));
        assert_eq!(shell_command("! ls -la"), Some("ls -la"));
        assert!(is_shell_command("!ls -la"));
    }

    #[test]
    fn a_leading_space_is_the_escape() {
        assert_eq!(shell_command(" !important, read this"), None);
        assert_eq!(shell_command("\t!tabbed"), None);
        assert!(!is_shell_command(" !important, read this"));
    }

    #[test]
    fn a_bare_bang_is_neither_command_nor_message() {
        assert_eq!(shell_command("!"), None);
        assert_eq!(shell_command("!   "), None);
        assert_eq!(shell_command("!\n\n"), None);
    }

    #[test]
    fn a_bang_anywhere_else_is_just_a_character() {
        assert_eq!(shell_command("what does ! do"), None);
        assert_eq!(shell_command("wow!"), None);
    }

    #[test]
    fn a_bang_starting_a_later_line_is_not_a_command() {
        // The whole buffer is the unit, not the line: this is a prompt that
        // happens to contain an exclamation-led line, and sending it must not
        // run anything.
        assert_eq!(shell_command("please run\n!rm -rf /"), None);
    }

    #[test]
    fn a_multi_line_command_keeps_every_line() {
        assert_eq!(shell_command("!echo first\necho second"),
                   Some("echo first\necho second"));
    }

    #[test]
    fn surrounding_whitespace_is_trimmed_from_the_command() {
        assert_eq!(shell_command("!  echo hi  \n"), Some("echo hi"));
    }

    #[test]
    fn an_empty_buffer_is_not_a_command() {
        assert_eq!(shell_command(""), None);
        assert!(!is_shell_command(""));
    }

    /// Every combination of the three flags the send control reads. Written
    /// out rather than looped because each line is a sentence from the spec,
    /// and a table would hide which one broke.
    #[test]
    fn a_shell_command_sends_whatever_the_session_is_doing() {
        use super::can_send;

        for (blocked, turn_active) in [(false, false), (true, false), (false, true), (true, true)] {
            assert!(can_send("!ls", blocked, turn_active),
                    "blocked={blocked} turn_active={turn_active}");
        }
    }

    #[test]
    fn an_ordinary_prompt_waits_for_the_session() {
        use super::can_send;

        assert!(can_send("hello", false, false));
        assert!(!can_send("hello", true, false),
                "a pending permission holds it");
        assert!(!can_send("hello", false, true), "a turn in flight holds it");
    }

    #[test]
    fn nothing_to_send_is_never_sendable() {
        use super::can_send;

        assert!(!can_send("", false, false));
        assert!(!can_send("   ", false, false));
        assert!(!can_send("!", false, false), "a bare bang is neither");
        assert!(!can_send("!  ", false, true));
    }
}
