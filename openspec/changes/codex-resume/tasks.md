# Tasks

## 1. Loading a session

- [x] 1.1 Keep the requested session id when a `session/load` response names
      none, in `knot-acp`'s `AcpClient::session_load`; verify with
      `client/tests.rs` for both response shapes.
- [x] 1.2 Record on `AcpSession` whether it resumed, gated on the connected
      adapter's advertised `loadSession`; verify with `acp_session` tests for
      a load, an adapter that cannot load, a refused load, and a fresh start.

## 2. First turns follow the outcome

- [x] 2.1 Build the registration and startup prompts for a fresh session in
      `ensure_panel_session`, and withhold both in `connect_into` only when the
      session resumed; verify with `panel_session/tests.rs` that a load sends
      no `session/prompt` and a refused load registers its fallback.

## 3. Codex

- [x] 3.1 Mark `codex` resume-capable in the adapter registry and replace
      `codex_resume_is_conservatively_unsupported`; verify `make test`.
- [ ] 3.2 Verify in the running app with `codex-acp` 2.0.0: a Codex agent
      survives a Knot restart with its history shown, sends no registration
      turn, and still follows its knot instructions and persona.
