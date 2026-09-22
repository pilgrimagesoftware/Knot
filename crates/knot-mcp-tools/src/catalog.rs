//! The tool catalogue and the dispatch that backs it: the `tools/list`
//! definitions and the `name -> handler` match `tools/call` runs.
//!
//! Split out of `lib.rs`, which held both alongside the catalogue struct
//! and its hook handler and was within a hundred lines of the project's
//! 700-line cap before the task tools were added. The two halves answer
//! different questions - what the server *is* versus what it *exposes* -
//! and only this half changes when a tool is added.
//!
//! Implements `openspec/specs/mcp-tools/spec.md`.

use async_trait::async_trait;
use knot_mcp::{PropertySchema, ToolCallResult, ToolCatalog, ToolDefinition, ToolInputSchema};

use crate::{McpToolCatalog, agents, consts, messaging, panels, repos, tasks};

fn mutates_agent_state(name: &str) -> bool {
    matches!(name,
             consts::REGISTER_AGENT
             | consts::CREATE_AGENT
             | consts::CLOSE_AGENT
             | consts::SET_STATUS
             | consts::DISPLAY_MARKDOWN
             | consts::VIEW_MERMAID)
}

fn prop(schema_type: &str, description: &str) -> PropertySchema {
    PropertySchema { schema_type: schema_type.to_string(),
                     description: description.to_string(), }
}

fn schema(properties: &[(&str, &str, &str)], required: &[&str]) -> ToolInputSchema {
    ToolInputSchema { properties:
                          properties.iter()
                                    .map(|(name, ty, desc)| ((*name).to_string(), prop(ty, desc)))
                                    .collect(),
                      required: required.iter().map(|s| s.to_string()).collect(),
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
              "List all registered agents with their status (name, folder, working/idle), \
               each with its description, capability tags, reachable tools and cost tier",
              &[("agentId", "string", "Your agent ID")],
              &["agentId"]),
         tool(consts::DESCRIBE_AGENTS,
              "Find agents by capability - answers 'who can do X'. Returns candidates \
               carrying every requested tag, cheapest-idle-first, across live agents and \
               deployable bench templates. Omit capabilities to list everything visible. \
               No match is an empty list, not an error.",
              &[("agentId", "string", "Your agent ID"),
                ("capabilities",
                 "array",
                 "Capability tags a candidate must all carry, e.g. [\"rust\", \"testing\"]. \
                  Omit to list every visible candidate."),
                ("includeTemplates",
                 "boolean",
                 "Include deployable bench templates as well as live agents (default true)")],
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
         tool(consts::PLAN_TASKS,
              "Commit a plan: a small graph of tasks with dependencies. Required before \
               dispatching work that spans more than one agent or more than one task. Cycles \
               and dangling dependencies are rejected. Re-planning keeps the state of tasks \
               already under way.",
              &[("agentId", "string", "Your agent ID"),
                ("tasks",
                 "array",
                 "Tasks as objects: id (unique in this plan), goal, and optionally assignee \
                  (an agent name or ID), capabilities (tags to resolve at dispatch time) and \
                  dependsOn (ids in this same plan)")],
              &["agentId", "tasks"]),
         tool(consts::DISPATCH_TASK,
              "Dispatch one ready task from your committed plan. Refused while any \
               dependency is not done, naming what it is waiting for.",
              &[("agentId", "string", "Your agent ID"),
                ("taskId", "string", "The task to dispatch")],
              &["agentId", "taskId"]),
         tool(consts::COMPLETE_TASK,
              "Report how a dispatched task turned out. Returns the tasks this made ready \
               and, on failure, the ones it blocked.",
              &[("agentId", "string", "Your agent ID"),
                ("taskId", "string", "The task to report on"),
                ("outcome", "string", "Either 'done' or 'failed'"),
                ("note", "string", "Optional note about the outcome")],
              &["agentId", "taskId", "outcome"]),
         tool(consts::TASK_STATUS,
              "Read your committed plan: every task with its state, assignee and \
               dependencies. Empty when you have not planned anything.",
              &[("agentId", "string", "Your agent ID")],
              &["agentId"]),
         tool(consts::VIEW_MERMAID,
              "Display a Mermaid diagram in a panel for the user to view. Supports flowcharts (graph TD/LR), state diagrams, sequence diagrams, class diagrams, and ER diagrams. Pass the mermaid source text directly. The diagram will be rendered natively alongside any open markdown panel.",
              &[("agentId", "string", "Your agent ID"),
                ("source", "string", "Mermaid diagram source text (e.g. 'graph TD; A-->B;')"),
                ("title", "string", "Optional title to display above the diagram")],
              &["agentId", "source"]),]
}

#[async_trait]
impl ToolCatalog for McpToolCatalog {
    fn list(&self) -> Vec<ToolDefinition> {
        tool_catalog()
    }

    async fn call(&self, name: &str, arguments: serde_json::Value) -> ToolCallResult {
        let result = match name {
            consts::REGISTER_AGENT => agents::register_agent(&mut self.agents.lock(), &arguments),
            consts::LIST_AGENTS => agents::list_agents(&self.agents.lock(), &arguments),
            consts::DESCRIBE_AGENTS => {
                let bench_agents = self.bench_agents.lock();
                agents::describe_agents(&self.agents.lock(), &bench_agents, &arguments)
            }
            consts::SEND_MESSAGE => messaging::send_message(&self.agents.lock(),
                                                            &mut self.messages.lock(),
                                                            self.notifier.as_ref(),
                                                            &arguments),
            consts::CHECK_MESSAGES => messaging::check_messages(&self.agents.lock(),
                                                                &mut self.messages.lock(),
                                                                &arguments),
            consts::BROADCAST_MESSAGE => messaging::broadcast_message(&self.agents.lock(),
                                                                      &mut self.messages.lock(),
                                                                      self.notifier.as_ref(),
                                                                      &arguments),
            consts::LIST_REPOS => repos::list_repos(&self.repos.borrow()),
            consts::LIST_WORKTREES => repos::list_worktrees(&self.repos.borrow(), &arguments),
            consts::CREATE_AGENT => {
                let mut bench_agents = self.bench_agents.lock();
                bench_agents.retain(|bench| std::path::Path::new(&bench.folder).is_dir());
                agents::create_agent(&mut self.agents.lock(), &arguments, &bench_agents)
            }
            consts::CLOSE_AGENT => agents::close_agent(&mut self.agents.lock(), &arguments),
            consts::CREATE_WORKTREE => repos::create_worktree(&arguments),
            consts::SET_STATUS => agents::set_status(&mut self.agents.lock(), &arguments),
            consts::DISPLAY_MARKDOWN => {
                panels::display_markdown(&mut self.agents.lock(), &arguments)
            }
            consts::VIEW_MERMAID => panels::view_mermaid(&mut self.agents.lock(), &arguments),
            consts::PLAN_TASKS => {
                tasks::plan_tasks(&self.agents.lock(), &mut self.graphs.lock(), &arguments)
            }
            consts::DISPATCH_TASK => {
                let bench_agents = self.bench_agents.lock();
                tasks::dispatch_task(tasks::DispatchContext { agents:   &self.agents.lock(),
                                                              graphs:   &mut self.graphs.lock(),
                                                              messages: &mut self.messages.lock(),
                                                              notifier: self.notifier.as_ref(),
                                                              bench:    &bench_agents, },
                                     &arguments)
            }
            consts::COMPLETE_TASK => {
                tasks::complete_task(&self.agents.lock(), &mut self.graphs.lock(), &arguments)
            }
            consts::TASK_STATUS => {
                tasks::task_status(&self.agents.lock(), &mut self.graphs.lock(), &arguments)
            }
            other => ToolCallResult::error(format!("unknown tool: {other}")),
        };
        if result.is_error.is_none()
           && mutates_agent_state(name)
           && let Err(error) = self.persist_agent_state()
        {
            return ToolCallResult::error(format!("Failed to persist agent state: {error}"));
        }
        result
    }
}
