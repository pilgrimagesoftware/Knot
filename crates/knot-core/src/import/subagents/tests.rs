//! Registry tests: which tools resolve to a reader, and what happens for one
//! that does not.

use std::str::FromStr;

use super::{SubagentTool, provider, supports_subagent_import};

#[test]
fn claude_resolves_to_a_provider() {
    assert!(supports_subagent_import(SubagentTool::Claude));
}

/// The three unread formats are unsupported rather than silently empty, per
/// the change's design note - tasks 4.1-4.3 land each one once its format has
/// been read from a real installation.
#[test]
fn tools_without_a_reader_are_unsupported() {
    for tool in [SubagentTool::Codex,
                 SubagentTool::OpenCode,
                 SubagentTool::Gemini]
    {
        assert!(!supports_subagent_import(tool), "{tool} has no reader yet");
        assert!(provider(tool).is_none(),
                "{tool} must not resolve to a reader");
    }
}

/// An unknown tool name never becomes a `SubagentTool` at all, so no read is
/// reachable for one.
#[test]
fn an_unknown_tool_name_does_not_parse() {
    for name in ["shell", "aider", ""] {
        assert!(SubagentTool::from_str(name).is_err(),
                "{name} should not be a known tool");
    }
}

#[test]
fn every_tool_round_trips_through_its_string_form() {
    for tool in SubagentTool::ALL {
        assert_eq!(SubagentTool::from_str(&tool.to_string()), Ok(tool));
    }
}

/// The Import tab lists only what it can actually read.
#[test]
fn only_implemented_tools_are_listed() {
    assert_eq!(SubagentTool::implemented(), vec![SubagentTool::Claude]);
}
