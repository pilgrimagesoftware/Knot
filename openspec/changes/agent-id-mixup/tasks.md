# Tasks

## 1. Launch

- [x] 1.1 Reproduce with a test: two agents sharing a folder and type, one with a persisted session, the history returning it. The second got it; the test fails on the old `resolve_resume_sessions`
- [x] 1.2 Resolve each session for at most one agent; history fallback only for an agent alone with its folder and type

## 2. Server

- [x] 2.1 `agent_mcp_url`; panel sessions connect on it
- [x] 2.2 Check against `claude-agent-acp` 0.82.0 that the `agent` query reaches every MCP request (`initialize`, `notifications/initialized`, `tools/list`)
- [x] 2.3 Bind MCP sessions to the agent their URL names; `Caller` and `ToolCatalog::call_as`
- [x] 2.4 Refuse calls naming another agent, and `register-agent` on an ID another live connection holds; bind on registration; tests through the catalog

## 3. Gate

- [x] 3.1 `make` passes
