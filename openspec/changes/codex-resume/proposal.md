# Proposal

## Why

Knot never resumed a Codex agent (issue #540). The adapter registry marked
`codex` `supports_resume: false`, calling resume "undocumented". `codex-acp`
2.0.0 does implement it: it advertises `loadSession`, and its `session/load`
resumes the Codex thread, replays the history and re-applies the
`CODEX_CONFIG` overrides that carry Knot's instructions and persona (#536).

Two things on Knot's side stood in the way, and fixing the flag alone would
have left both:

- `session/load`'s response has no `sessionId` in ACP, and `codex-acp` sends
  none. Knot's client treated the missing id as an error, so every load looked
  like a failure and fell back to `session/new`.
- Whether to send the registration and startup prompts was decided by whether
  a prior session id was *stored*, before the adapter was even connected. An
  adapter that could not load - an older `codex-acp`, or a session it no
  longer has - left a fresh session that was never registered.

## What Changes

- `session/load` keeps the id it was asked to load when the response names
  none, and takes the adapter's id when it does.
- A connected session records whether it actually resumed. The registration
  and startup prompts are withheld only then; a fallback `session/new` is sent
  both, like any fresh session.
- `codex` is marked resume-capable in the adapter registry, so it reports the
  `resume` tag. The resume itself stays gated on the connected adapter's
  advertised `loadSession`, so an older `codex-acp` stays fresh.

## Impact

- `knot-acp`: `AcpClient::session_load`.
- `knot-terminal`: `AcpSession::resumed`.
- `knot`: `panel_session::connect_into`, and `ensure_panel_session`, which now
  always builds the fresh-session prompts.
- `knot-agent-launch`: the `codex` registry entry.
- Specs: `acp-client`, `agent-launch-command`.
