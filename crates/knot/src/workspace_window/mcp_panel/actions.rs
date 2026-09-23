//! Handing the user an agent's own MCP flow.
//!
//! Contract: `openspec/specs/agent-mcp-status/spec.md` - "A server needing
//! attention delegates to the agent's own flow".
//!
//! Knot does not authenticate anything. It opens a plain shell in the agent's
//! working directory - a shell companion, the same one the sidebar's "New
//! Shell Companion" creates, which is what `acp-panel-ui`'s "Terminal remains
//! available" already guarantees - and runs that agent type's own MCP command
//! in it. The OAuth round-trip, the device code and the server's own prompts
//! all happen where they already work.
//!
//! The ACP session is untouched. The adapter subprocess is never the
//! injection target: it speaks JSON-RPC, not shell, and typing at it would
//! corrupt the session the user is trying to repair.

use uuid::Uuid;

use super::handover::handover;
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// The binary this agent's CLI is invoked as: the user's configured
    /// command when they set one, otherwise the type's default.
    ///
    /// The configured value may carry flags meant for launching the agent
    /// (`claude --resume`); its first token is the binary, and the rest is
    /// not an argument to an MCP command.
    pub(in crate::workspace_window) fn mcp_program_for(&self, agent_type: &str,
                                                       cx: &gpui_kit::App)
                                                       -> Option<String> {
        crate::settings_global::read(cx).agent_commands
                                        .get(agent_type)
                                        .and_then(|text| text.split_whitespace().next())
                                        .map(str::to_owned)
                                        .or_else(|| {
                                            knot_core::agent_type::mcp_program(agent_type).map(str::to_owned)
                                        })
    }

    /// Opens a terminal running this agent type's MCP flow for `server`.
    ///
    /// Does nothing for a type with no MCP command - a row on such a type
    /// offers no action in the first place, and this is the second half of
    /// that same check.
    pub(in crate::workspace_window) fn open_mcp_handover(&mut self, agent_id: Uuid,
                                                         server: &str, cx: &gpui_kit::App)
                                                         -> bool {
        let Some(agent_type) = self.store
                                   .lock()
                                   .agent(agent_id)
                                   .map(|agent| agent.agent_type.clone())
        else {
            return false;
        };

        let Some(program) = self.mcp_program_for(&agent_type, cx)
        else {
            return false;
        };

        let Some(flow) = handover(&agent_type, &program, server)
        else {
            return false;
        };

        // The same shape `create_shell_companion` builds - a shell, in the
        // owner's folder, inserted beside it and inheriting its activation -
        // written out here because this one carries a name and a command of
        // its own, and `CreateOptions` is where those belong.
        //
        // `shell_command` becomes the PTY's initialization command, which is
        // how the flow is entered without typing at anything already running.
        let companion = {
            let mut store = self.store.lock();
            let Some(owner) = store.agent(agent_id)
            else {
                return false;
            };

            if owner.is_companion {
                return false;
            }

            let (folder, activation_mode, activated) =
                (owner.folder.clone(), owner.activation_mode, owner.activated);

            let companion =
                store.create(folder,
                             knot_agents::CreateOptions { name: Some(knot_core::l10n::t("mcp.terminal_name")),
                                                          agent_type:
                                                              Some(knot_core::agent_type::SHELL.to_owned()),
                                                          shell_command: Some(flow.command().to_owned()),
                                                          created_by: Some(agent_id),
                                                          is_companion: true,
                                                          insert_after: Some(agent_id),
                                                          activation_mode,
                                                          ..Default::default() });
            store.set_activated(companion, activated);

            companion
        };

        self.persist_agents(cx);
        // So the section catches up with whatever the user did in there.
        self.mcp_handover_terminals.insert(companion, agent_id);

        true
    }

    /// Notices a delegated terminal exiting, and asks for a fresh probe.
    ///
    /// Called from the repaint poll's exit loop, where every other
    /// consequence of a session ending is already handled. Returns whether
    /// this was one of ours, so the caller can tell a delegated terminal from
    /// an ordinary companion.
    pub(in crate::workspace_window) fn finish_mcp_handover(&mut self, exited: Uuid) -> bool {
        let Some(owner) = self.mcp_handover_terminals.remove(&exited)
        else {
            return false;
        };

        // Unconditional: a terminal closed without the user doing anything
        // re-probes to the same answer, which is cheaper than working out
        // whether anything changed and cannot leave the section stale.
        self.request_mcp_probe(owner);

        true
    }
}

#[cfg(test)]
mod tests;
