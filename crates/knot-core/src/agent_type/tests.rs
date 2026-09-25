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
    assert_eq!(subagent_reporting("nothing-by-that-name"),
               SubagentReporting::None);
}

/// The column that decides whether the processes section shows a subagents
/// group at all. A row left off the roster would read as "dispatched none",
/// which is the one answer this capability must never give by accident.
#[test]
fn every_type_states_how_it_reports_subagents() {
    for agent_type in ALL {
        // Reading the field is the assertion: the struct has no default, so a
        // row that omitted it would not compile. This test exists so that
        // removing the column from a populated row fails here too, rather
        // than only wherever the column happens to be read.
        let _ = agent_type.subagents;
    }

    assert_eq!(subagent_reporting("claude"), SubagentReporting::ToolCalls);
}

/// A shell agent has no AI and so nothing to delegate. It must resolve to
/// "cannot tell" structurally, not by anybody remembering to special-case it.
#[test]
fn a_shell_type_reports_no_subagents_in_either_view_mode() {
    assert_eq!(subagent_reporting(SHELL), SubagentReporting::None);
    assert!(!reports_subagents(SHELL, ViewMode::Terminal));
    assert!(!reports_subagents(SHELL, ViewMode::Panel));
}

/// The sequencing decision from design.md. Claude ships as `ToolCalls`, not
/// `Either`, because the hook emitter is a plugin outside this repo - so a
/// Terminal-mode Claude agent must read as *unavailable* rather than
/// confidently claiming it dispatched nothing.
#[test]
fn claude_reports_subagents_in_panel_mode_only_until_the_hook_plugin_ships() {
    assert!(reports_subagents("claude", ViewMode::Panel));
    assert!(!reports_subagents("claude", ViewMode::Terminal));
}

#[test]
fn each_reporting_path_answers_for_its_own_view_mode() {
    assert!(SubagentReporting::ToolCalls.can_report(ViewMode::Panel));
    assert!(!SubagentReporting::ToolCalls.can_report(ViewMode::Terminal));

    assert!(SubagentReporting::Hooks.can_report(ViewMode::Terminal));
    assert!(!SubagentReporting::Hooks.can_report(ViewMode::Panel));

    assert!(SubagentReporting::Either.can_report(ViewMode::Panel));
    assert!(SubagentReporting::Either.can_report(ViewMode::Terminal));

    assert!(!SubagentReporting::None.can_report(ViewMode::Panel));
    assert!(!SubagentReporting::None.can_report(ViewMode::Terminal));
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
        assert!(mcp_list_args(id).is_some(),
                "{id} is listed as probeable but has no command");
    }

    for id in UNPROBEABLE {
        assert!(mcp_list_args(id).is_none(),
                "{id} is listed as unprobeable but has a command");
    }
}

/// A row that can be listed but not acted on is a dead end: the section
/// would show a server needing attention and offer nothing to do about it.
#[test]
fn anything_that_can_be_listed_can_also_be_managed() {
    for agent_type in ALL {
        if !agent_type.mcp_list_args.is_empty() {
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
    assert_eq!(mcp_list_args("shell"), None);
    assert!(!mcp_manage("shell").is_available());
}

/// An unrecognized type gets the same answer as a known-unprobeable one, and
/// it means the same thing: not "no servers", but "cannot find out".
#[test]
fn an_unknown_type_cannot_be_probed_or_managed() {
    assert_eq!(mcp_list_args("nothing-by-that-name"), None);
    assert!(!mcp_manage("nothing-by-that-name").is_available());
}
