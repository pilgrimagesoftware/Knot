use async_trait::async_trait;
use knot_mcp::{PropertySchema, ToolCallResult, ToolCatalog, ToolDefinition, ToolInputSchema};

use crate::McpToolCatalog;
use crate::agents;
use crate::consts;
use crate::{messaging, panels, repos};

fn prop(schema_type: &str, description: &str) -> PropertySchema {
    PropertySchema { schema_type: schema_type.to_string(),
                     description: description.to_string(), }
}

fn schema(properties: &[(&str, &str, &str)], required: &[&str]) -> ToolInputSchema {
    ToolInputSchema { properties:
                          properties.iter()
                                    .map(|(name, ty, desc)| ((*name).to_string(), prop(ty, desc)))
                                    .collect(),
                      required: required.iter().map(|value| value.to_string()).collect(),
                      ..Default::default() }
}

fn tool(name: &str, description: &str, properties: &[(&str, &str, &str)], required: &[&str])
        -> ToolDefinition {
    ToolDefinition { name:         name.to_string(),
                     description:  description.to_string(),
                     input_schema: schema(properties, required), }
}

fn tool_catalog() -> Vec<ToolDefinition> {
    vec![tool(consts::REGISTER_AGENT,
              "Register this agent with Knot crew. Call this first before using other tools.",
              &[("agentId", "string", "The agent ID provided by Knot"),
                ("sessionId", "string", "Your internal session ID.")],
              &["agentId"]),
         tool(consts::LIST_AGENTS,
              "List all registered agents with their status (name, folder, working/idle)",
              &[("agentId", "string", "Your agent ID")],
              &["agentId"]),
         tool(consts::SEND_MESSAGE,
              "Send a message to another agent by name or ID",
              &[("from", "string", "Your agent ID"),
                ("to", "string", "Recipient agent name or ID"),
                ("content", "string", "Message content")],
              &["from", "to", "content"]),
         tool(consts::CHECK_MESSAGES,
              "Check your inbox for new messages from other agents",
              &[("agentId", "string", "Your agent ID"),
                ("markAsRead", "boolean", "Mark messages as read (default: true)")],
              &["agentId"]),
         tool(consts::BROADCAST_MESSAGE,
              "Send a message to all other registered agents",
              &[("from", "string", "Your agent ID"),
                ("content", "string", "Message content")],
              &["from", "content"]),
         tool(consts::LIST_REPOS,
              "List all git repositories in the configured source folder",
              &[],
              &[]),
         tool(consts::LIST_WORKTREES,
              "List all worktrees for a given repository",
              &[("repoPath", "string", "Path to the repository")],
              &["repoPath"]),
         tool(consts::CREATE_AGENT,
              "Create a new agent in Knot. Can optionally create a new git worktree for the agent. Note: shell agents are plain terminals without an AI agent, so do not try to send messages to them.",
              &[("agentId", "string", "Your agent ID (used to track who created the agent)"),
                ("benchAgentId",
                 "string",
                 "ID of a bench agent to deploy. When provided, name/agentType/repoPath are optional and default to the bench agent's configuration."),
                ("name", "string", "Name for the agent"),
                ("icon", "string", "Emoji icon for the agent (e.g., '🤖')"),
                ("agentType",
                 "string",
                 "Agent type: claude, codex, opencode, gemini, copilot, custom1, custom2, or shell"),
                ("repoPath", "string", "Path to the repository or worktree folder"),
                ("createWorktree", "boolean", "If true, create a new worktree from repoPath"),
                ("branchName",
                 "string",
                 "Branch name for new worktree (required if createWorktree is true)"),
                ("companion",
                 "boolean",
                 "If true, the new agent is a companion of the creator: it won't appear in the agent list, its visibility is linked to the creator, and it will be closed when the creator is closed. Only use this flag if the user has explicitly asked for a companion agent."),
                ("command", "string", "Command to run (only for shell agent type)"),
                ("personaId",
                 "string",
                 "ID of a persona to apply. Only works with agents that support system prompts (claude, codex).")],
              &["agentId"]),
         tool(consts::CLOSE_AGENT,
              "Close an agent that you created. You can only close agents that you created, not agents created by the user or other agents.",
              &[("agentId", "string", "Your agent ID"),
                ("target", "string", "The agent to close (name or ID)")],
              &["agentId", "target"]),
         tool(consts::CREATE_WORKTREE,
              "Create a new git worktree from a repository. Returns the path to the new worktree.",
              &[("repoPath", "string", "Path to the source repository"),
                ("branchName", "string", "Branch name for the new worktree")],
              &["repoPath", "branchName"]),
         tool(consts::SET_STATUS,
              "MANDATORY: Set your status so other agents know what you are doing. Call before starting any task, after completing it, and when changing direction. Keep it short and specific (e.g. 'Implementing auth module', 'Running tests', 'Done — PR ready'). Use empty string to clear.",
              &[("agentId", "string", "Your agent ID"),
                ("status",
                 "string",
                 "Short status text describing what you are currently doing. Use empty string to clear.")],
              &["agentId", "status"]),
         tool(consts::DISPLAY_MARKDOWN,
              "Display a markdown file in a panel for the user to review. Use this to show plans, documentation, or any markdown content that needs user attention. Also use if the user asks you to show him a file. Never assume the panel is open or displaying the right file as the user may have closed it: call the tool again when relevant.",
              &[("agentId", "string", "Your agent ID"),
                ("filePath", "string", "Absolute path to the markdown file to display"),
                ("maximized",
                 "boolean",
                 "If true, maximize the panel to fill the available space. Only set to true if the user explicitly requests it. Default: false")],
              &["agentId", "filePath"]),
         tool(consts::VIEW_MERMAID,
              "Display a Mermaid diagram in a panel for the user to view. Supports flowcharts (graph TD/LR), state diagrams, sequence diagrams, class diagrams, and ER diagrams. Pass the mermaid source text directly. The diagram will be rendered natively alongside any open markdown panel.",
              &[("agentId", "string", "Your agent ID"),
                ("source", "string", "Mermaid diagram source text (e.g. 'graph TD; A-->B;')"),
                ("title", "string", "Optional title to display above the diagram")],
              &["agentId", "source"]),]
}

fn mutates_agent_state(name: &str) -> bool {
    matches!(name,
             consts::REGISTER_AGENT
             | consts::CREATE_AGENT
             | consts::CLOSE_AGENT
             | consts::SET_STATUS
             | consts::DISPLAY_MARKDOWN
             | consts::VIEW_MERMAID)
}

#[async_trait]
impl ToolCatalog for McpToolCatalog {
    fn list(&self) -> Vec<ToolDefinition> {
        tool_catalog()
    }

    async fn call(&self, name: &str, arguments: serde_json::Value) -> ToolCallResult {
        let result = match name {
            consts::REGISTER_AGENT => {
                agents::register_agent(&mut self.agents.lock().unwrap(), &arguments)
            }
            consts::LIST_AGENTS => agents::list_agents(&self.agents.lock().unwrap(), &arguments),
            consts::SEND_MESSAGE => messaging::send_message(&self.agents.lock().unwrap(),
                                                            &mut self.messages.lock().unwrap(),
                                                            self.notifier.as_ref(),
                                                            &arguments),
            consts::CHECK_MESSAGES => messaging::check_messages(&self.agents.lock().unwrap(),
                                                                &mut self.messages.lock().unwrap(),
                                                                &arguments),
            consts::BROADCAST_MESSAGE => {
                messaging::broadcast_message(&self.agents.lock().unwrap(),
                                             &mut self.messages.lock().unwrap(),
                                             self.notifier.as_ref(),
                                             &arguments)
            }
            consts::LIST_REPOS => repos::list_repos(&self.repos.borrow()),
            consts::LIST_WORKTREES => repos::list_worktrees(&self.repos.borrow(), &arguments),
            consts::CREATE_AGENT => {
                let mut bench_agents = self.bench_agents.lock().unwrap();
                bench_agents.retain(|bench| std::path::Path::new(&bench.folder).is_dir());
                agents::create_agent(&mut self.agents.lock().unwrap(), &arguments, &bench_agents)
            }
            consts::CLOSE_AGENT => {
                agents::close_agent(&mut self.agents.lock().unwrap(), &arguments)
            }
            consts::CREATE_WORKTREE => repos::create_worktree(&arguments),
            consts::SET_STATUS => agents::set_status(&mut self.agents.lock().unwrap(), &arguments),
            consts::DISPLAY_MARKDOWN => {
                panels::display_markdown(&mut self.agents.lock().unwrap(), &arguments)
            }
            consts::VIEW_MERMAID => {
                panels::view_mermaid(&mut self.agents.lock().unwrap(), &arguments)
            }
            other => ToolCallResult::error(format!("unknown tool: {other}")),
        };
        if result.is_error.is_none() && mutates_agent_state(name) {
            if let Err(error) = self.persist_agent_state() {
                return ToolCallResult::error(format!("Failed to persist agent state: {error}"));
            }
        }
        result
    }
}
