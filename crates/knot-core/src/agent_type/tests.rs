//! Unit tests for [`super`].

use super::*;

/// An id typed twice is two types that behave as one, which is exactly the
/// confusion the roster exists to prevent.
#[test]
fn every_id_appears_once() {
    let mut ids = ALL.iter()
                     .map(|agent_type| agent_type.id)
                     .collect::<Vec<_>>();
    ids.sort_unstable();
    let unique = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), unique);
}

#[test]
fn the_default_and_shell_ids_are_in_the_roster() {
    assert!(info(DEFAULT).is_some());
    assert!(is_shell(SHELL));
}

/// Exactly one type is a shell: the view mode, the launch path and the
/// tracking preset all branch on it, so a second one would have to be
/// taught to each of them separately.
#[test]
fn only_one_type_is_a_shell() {
    assert_eq!(ALL.iter().filter(|agent_type| agent_type.is_shell).count(),
               1);
}

/// A custom type is a user's own command, so nothing may assume vendor
/// behaviour of it - no inline registration, no hook-driven activity.
#[test]
fn a_custom_type_claims_no_vendor_behaviour() {
    for agent_type in ALL.iter().filter(|agent_type| agent_type.is_custom) {
        assert!(!agent_type.inline_registration,
                "{} claims inline registration",
                agent_type.id);
        assert!(!agent_type.hook_activity,
                "{} claims hook activity",
                agent_type.id);
    }
}

/// An unrecognized type is a working configuration - it launches through
/// the terminal path - so every lookup has to answer for one rather than
/// panicking or pretending it is the default.
#[test]
fn an_unknown_type_answers_as_itself() {
    assert_eq!(info("nothing-by-that-name"), None);
    assert_eq!(label("nothing-by-that-name"), "nothing-by-that-name");
    assert!(!is_shell("nothing-by-that-name"));
    assert!(!supports_inline_registration("nothing-by-that-name"));
    assert!(!has_hook_activity("nothing-by-that-name"));
}

#[test]
fn a_known_type_answers_from_its_row() {
    assert_eq!(label("opencode"), "OpenCode");
    assert!(supports_inline_registration("gemini"));
    assert!(has_hook_activity("claude"));
    assert!(!has_hook_activity("gemini"));
}

/// The ids Knot can ask about their MCP servers today: the ones whose real
/// listing output has been captured and has a parser pinned to a fixture.
const PROBEABLE: &[&str] = &["claude", "gemini"];

/// The ids that deliberately cannot be asked. Keeping them written down is
/// the point: a new agent type added without an MCP decision changes this
/// count and fails, rather than quietly inheriting "no MCP support" and
/// telling its users nothing.
///
/// Why each one:
///
/// - `codex` - `codex mcp` exists; its output shape is unobserved.
/// - `opencode` - `opencode mcp list` exists; its *populated* shape is
///   unobserved. Its per-server handover is known and already recorded.
/// - `copilot` - no MCP listing command is known.
/// - `custom1`, `custom2` - a user-configured command, not a vendor CLI.
/// - `shell` - runs no MCP client at all.
const UNPROBEABLE: &[&str] = &["codex", "opencode", "copilot", "custom1", "custom2", "shell"];

/// Adding an agent type must force a decision about its MCP support. Without
/// this, a new row defaults to "cannot determine" and nobody finds out until
/// a user asks why their agent's servers are not listed.
#[test]
fn every_known_type_has_a_deliberate_mcp_answer() {
    assert_eq!(PROBEABLE.len() + UNPROBEABLE.len(),
               ALL.len(),
               "an agent type was added or removed without an MCP decision; update PROBEABLE or \
                UNPROBEABLE");

    for id in PROBEABLE {
        assert!(mcp_list_command(id).is_some(),
                "{id} is listed as probeable but has no command");
    }

    for id in UNPROBEABLE {
        assert!(mcp_list_command(id).is_none(),
                "{id} is listed as unprobeable but has a command");
    }
}

/// A row that can be listed but not acted on is a dead end: the section
/// would show a server needing attention and offer nothing to do about it.
#[test]
fn anything_that_can_be_listed_can_also_be_managed() {
    for agent_type in ALL {
        if !agent_type.mcp_list_command.is_empty() {
            assert!(agent_type.mcp_manage.is_available(),
                    "{} can be listed but offers no handover",
                    agent_type.id);
        }
    }
}

/// The two handover shapes, each on the type that actually has it.
#[test]
fn handover_shape_matches_what_the_cli_offers() {
    assert!(matches!(mcp_manage("claude"), McpManage::Interactive { send, .. } if send == "/mcp"),
            "Claude Code has no per-server command; its /mcp UI is the handover");

    let McpManage::PerServer(command) = mcp_manage("opencode")
    else {
        panic!("`opencode mcp auth <name>` addresses one server directly")
    };
    assert!(command.contains(&"%{server}"),
            "a per-server command must name the server");

    assert_eq!(mcp_manage("shell"), McpManage::None);
}

/// A bare shell runs no MCP client, so offering it either column would be
/// offering something that cannot work.
#[test]
fn a_shell_has_no_mcp_support_at_all() {
    assert_eq!(mcp_list_command("shell"), None);
    assert!(!mcp_manage("shell").is_available());
}

/// An unrecognized type gets the same answer as a known-unprobeable one, and
/// it means the same thing: not "no servers", but "cannot find out".
#[test]
fn an_unknown_type_cannot_be_probed_or_managed() {
    assert_eq!(mcp_list_command("nothing-by-that-name"), None);
    assert!(!mcp_manage("nothing-by-that-name").is_available());
}
