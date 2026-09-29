# Tasks

## 1. Options to the adapters

- [x] 1.1 A table-driven flag splitter (`agent_options/flags.rs`) that forwards only the flags an adapter's table names, and drops an unknown flag together with its value
- [x] 1.2 Codex options as `CODEX_CONFIG` overrides (`-c` with dotted keys and TOML-like values, `--model`, `--profile`, `--enable`, `--disable`), with tests
- [x] 1.3 OpenCode options as `OPENCODE_CONFIG_CONTENT` `model`/`default_agent` plus `acp` argv switches, with tests
- [x] 1.4 Gemini options as extra argv from an allowlist, with tests
- [x] 1.5 `AdapterOptions::layered_env`: options over the inherited value, under the standing instructions; keep the variable through the opencode instruction fallbacks
- [x] 1.6 Carry extra argv through `ConnectRequest` and `SessionTarget`
- [x] 1.7 Live check: `CODEX_CONFIG` `model_reasoning_effort=high` opened a `codex-acp` 2.0.0 session at `high` (default `medium`); `default_agent=plan` opened an `opencode acp` 1.18.30 session in `plan` (default `build`) with `--print-logs --log-level WARN`; `gemini --acp --skip-trust --model … --approval-mode plan --include-directories /tmp` 0.46.0 initialized, while an unknown flag made it exit
- [x] 1.8 `make` passes
