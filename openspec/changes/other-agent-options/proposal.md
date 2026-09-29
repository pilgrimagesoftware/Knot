# Proposal

## Why

Options set in Settings - Coding - Agent Options for Codex, OpenCode, Gemini
and Copilot never reach those agents (#532). The `claude-agent-options` change
passed Claude's through `_meta.claudeCode.options.extraArgs`, and the other
types are logged as "options not passed to its adapter". Each of those
adapters takes options in its own way, and appending them to the command line
blindly would break the launch: `codex-acp` never passes its argv on to
Codex, and `gemini` exits on a flag it does not know.

## What Changes

- **Codex**: `-c key=value`, `--model`, `--profile`, `--enable` and `--disable`
  become config overrides in the adapter's `CODEX_CONFIG`, the same JSON object
  the standing instructions go in. `codex-acp` sends it on every `thread/start`
  and `thread/resume`.
- **OpenCode**: `--model` and `--agent` become `model` and `default_agent` in
  `OPENCODE_CONFIG_CONTENT`. `--print-logs`, `--log-level` and `--pure` are
  passed on the `opencode acp` command line.
- **Gemini**: a fixed set of flags that leave an ACP session an ACP session
  (`--model`, `--approval-mode`, `--yolo`, `--sandbox`, `--include-directories`,
  `--extensions`, policy and tool lists) is appended to `gemini --acp
  --skip-trust`, under the flags' long names.
- The options sit between Knot's inherited environment and the standing
  instructions: they override what the environment set, and the instructions
  are merged in afterwards, so no option can displace them.
- Anything else - a flag an adapter owns or would be changed by, a stray
  word, all of it for Copilot - is logged as not passed.

## Non-Goals

- Copilot. Its CLI is not installed on the machine this was checked on, so
  its flags are unconfirmed and its options stay unforwarded.
- Mapping Codex's `-s`/`-a` onto the session's mode. `codex-acp` sets
  approval and sandbox policy on every turn from the mode it offers, so they
  would be overwritten; the mode control already covers them.
