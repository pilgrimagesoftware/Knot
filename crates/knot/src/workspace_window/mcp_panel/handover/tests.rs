use super::{Handover, handover};

/// Claude Code has no per-server command, so the handover is its own `/mcp`
/// UI: run the CLI, then send the command to it.
#[test]
fn an_interactive_type_runs_the_cli_and_sends_its_command() {
    let flow = handover("claude", "claude", "github").expect("claude has a flow");

    assert_eq!(flow,
               Handover::Interactive { command: "claude".to_owned(),
                                       send:    "/mcp".to_owned(), });
    assert_eq!(flow.send(), Some("/mcp"));
}

/// OpenCode addresses one server directly, so the user lands on the server
/// they clicked rather than having to find it in a list.
#[test]
fn a_per_server_type_names_the_server_on_the_command_line() {
    let flow = handover("opencode", "opencode", "github").expect("opencode has a flow");

    assert_eq!(flow.command(), "opencode mcp auth 'github'");
    assert_eq!(flow.send(), None, "there is nothing left to navigate");
}

/// The user's configured command is the binary their agent actually runs, so
/// the flow opens in the same installation the probe read from.
#[test]
fn the_configured_program_is_what_runs() {
    let flow = handover("claude", "/opt/custom/claude", "github").expect("a flow");

    assert_eq!(flow.command(), "/opt/custom/claude");
}

#[test]
fn a_type_with_no_mcp_command_offers_nothing() {
    assert_eq!(handover("shell", "zsh", "github"), None);
    assert_eq!(handover("codex", "codex", "github"), None);
    assert_eq!(handover("nothing-by-that-name", "x", "github"), None);
}

/// Shell metacharacters are neutralized by quoting rather than rejected -
/// they cannot escape a single-quoted word, and rejecting them would turn a
/// legal name into no action at all.
#[test]
fn shell_metacharacters_are_quoted_not_executed() {
    for name in ["a;rm -rf /",
                 "$(whoami)",
                 "`id`",
                 "a|b",
                 "a&b",
                 "a>b",
                 "../../etc/passwd"]
    {
        let flow = handover("opencode", "opencode", name).expect("still a flow");

        // The whole name inside one quoted word, and no quote anywhere else:
        // nothing in it can reach the shell as syntax.
        assert_eq!(flow.command(),
                   format!("opencode mcp auth '{name}'"),
                   "for {name:?}");
    }
}

/// The one case quoting cannot fix. This text is typed into a PTY and
/// followed by Return, so a newline inside the name submits the line early
/// and the remainder runs as its own command.
#[test]
fn a_newline_never_reaches_the_command_line() {
    for name in ["github\nrm -rf /", "github\r\nid", "a\tb", "a\0b"] {
        let flow = handover("opencode", "opencode", name).expect("the fallback");

        assert_eq!(flow.command(),
                   "opencode",
                   "{name:?} should have fallen back to the bare CLI");
        assert!(!flow.command().contains('\n'),
                "a newline reached the command: {flow:?}");
        assert!(!flow.command().contains('\r'));
        assert!(!flow.command().contains("rm -rf"));
    }
}

/// A single quote inside the name must close and reopen the quoting rather
/// than ending it - the only form that is total for POSIX shells.
#[test]
fn an_embedded_single_quote_is_escaped_rather_than_ending_the_quoting() {
    let flow = handover("opencode", "opencode", "it's-mine").expect("a flow");

    assert_eq!(flow.command(), r"opencode mcp auth 'it'\''s-mine'");
}

/// Real names hold spaces, dots, colons and parentheses. A pattern tight
/// enough to feel reassuring would reject most of the servers a user has.
#[test]
fn ordinary_names_with_punctuation_are_accepted() {
    for name in ["claude.ai Asana (2)",
                 "plugin:kochava:github",
                 "my-server_1",
                 "server@host"]
    {
        let flow = handover("opencode", "opencode", name).expect("a flow");

        assert!(matches!(flow, Handover::Command { .. }),
                "{name:?} was rejected: {flow:?}");
        assert!(flow.command().contains(name),
                "{name:?} did not survive: {}",
                flow.command());
    }
}

/// An empty or absurd name is not something to type at a shell.
#[test]
fn an_empty_or_enormous_name_falls_back() {
    for name in [String::new(), "x".repeat(500)] {
        let flow = handover("opencode", "opencode", &name).expect("the fallback");

        assert_eq!(flow.command(), "opencode", "got {flow:?}");
    }
}

/// The fallback still opens the agent, in the right folder - one step further
/// from the server than naming it would have been, which is the cost of not
/// typing a name we are unwilling to quote. Offering nothing would strand the
/// user entirely, which is the state this whole change exists to end.
#[test]
fn the_fallback_still_opens_the_agents_own_cli() {
    let flow = handover("opencode", "opencode", "bad\nname").expect("a flow");

    assert_eq!(flow.command(), "opencode");
    assert_eq!(flow.send(),
               None,
               "there is no interactive command recorded for this type");
}
