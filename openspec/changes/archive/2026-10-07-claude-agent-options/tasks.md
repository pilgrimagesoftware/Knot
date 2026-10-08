# Tasks

## 1. Options to the adapter

- [x] 1.1 Add `knot_agent_launch::adapter_options`, parsing a type's options into `_meta.claudeCode.options.extraArgs`, a starting mode and the ignored words, with tests
- [x] 1.2 Send a session `_meta` on `session/new` and `session/load` (`AcpClient::with_session_meta`), with tests
- [x] 1.3 Carry it through `AcpSession::start` via a `SessionTarget`, with a test that the adapter receives it
- [x] 1.4 Read `agent_options` in `ensure_panel_session`; apply the options' mode over the persisted and default modes; log ignored words
- [x] 1.5 Include `data.details` in ACP RPC error messages
- [x] 1.6 Check against `claude-agent-acp` 0.82.0 that `extraArgs` reach `claude`'s argv: `--add-dir /tmp/knot501 --remote-control` appeared on the spawned process and the session opened; an unknown flag failed `session/new` with the CLI's message in `data.details`
- [x] 1.7 `make` passes
