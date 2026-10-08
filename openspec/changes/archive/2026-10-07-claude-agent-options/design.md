# Design

## Context

`claude-agent-acp` spawns `claude` through the Claude Agent SDK. Its argv
carries no pass-through; per session it reads `params._meta.claudeCode.options`
and spreads it into the SDK's `Options`, of which `extraArgs` (flag name ->
value, or `null` for a bare flag) is written onto `claude`'s command line. It
always passes `--permission-mode` itself, from `permissions.defaultMode`, and
`--allow-dangerously-skip-permissions` unless running as root. Checked live
against 0.82.0: `extraArgs {"add-dir": "/tmp/x", "remote-control": null}`
yields `claude ... --add-dir /tmp/x --remote-control`, and an unknown flag
fails `session/new` with `-32603 Internal error`, the CLI's stderr in
`data.details`.

## Decisions

### Parse the options text into `extraArgs`

Split with `shell-words` so quoted values stay whole. `--name=value` and
`--name value` (the next word, if it is not a flag) give a value; a lone
`--name` gives `null`. `extraArgs` is a map, so a repeated flag keeps its last
value. Unbalanced quoting forwards nothing rather than a guess.

### Permission flags become the starting mode

Forwarding `--dangerously-skip-permissions` beside the adapter's own
`--permission-mode default` leaves which one wins to the CLI. The session's
`mode` config option is the adapter's supported switch, and the #516 path
already sets it after connect, so the flag maps to `bypassPermissions` and
`--permission-mode` to its mode id through the adapter's alias table. This mode
wins over a persisted panel choice and over settings files, as the flag does on
every CLI launch; a mode the adapter does not offer (bypass as root) is not
requested.

### Flags the adapter owns are dropped

A user copy of `--output-format`, `--input-format`, `--resume` and the like
would change the stream or the session under the adapter. These are listed in
`consts.rs` and skipped, like short flags (`extraArgs` can only write `--name`)
and stray words. Dropped words are logged.

### Plumbing

`_meta` rides on `AcpClient` (`with_session_meta`) so `session/new` and
`session/load` both send it without new parameters. `AcpSession::start` takes
a `SessionTarget` struct (cwd, prior session, MCP URL, meta), which keeps it
under the argument limit and removes a `too_many_arguments` allow.

### Show the adapter's error details

An unknown flag now fails the session. `JsonRpcErrorPayload::described` appends
`data.details` to the message, so the failed panel names the option.

## Risks / Trade-offs

- [`--flag word` where `word` was meant as a positional] -> positionals have no
  meaning to an ACP-driven `claude`; the word becomes the flag's value.
- [The adapter renames `_meta.claudeCode.options`] -> options stop applying,
  no crash; the adapter test in this change's tasks is the check to repeat on
  an adapter bump.
