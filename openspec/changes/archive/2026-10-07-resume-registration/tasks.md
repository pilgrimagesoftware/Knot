# Tasks

## 1. Server

- [x] 1.1 `ToolCatalog::connected`, called by the server on every request whose URL names an agent; HTTP test that initialize on a bound URL reports it and an unbound one reports nobody
- [x] 1.2 `McpToolCatalog::connected` marks a known, unregistered agent registered; test

## 2. Resume

- [x] 2.1 `keeps_mcp_query` (claude, codex) and `resume_registration_prompt`; tests
- [x] 2.2 `ConnectRequest.resume_registration_prompt`, sent only when the session really resumed; resume-level test that a loaded session gets exactly one turn where needed, and none otherwise

## 3. Gate

- [x] 3.1 `make` passes
