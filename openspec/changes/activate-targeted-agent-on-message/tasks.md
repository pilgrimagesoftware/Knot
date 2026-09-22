# Tasks

## 1. Activation queue plumbing

- [ ] 1.1 Add `ActivationQueue` (`Arc<Mutex<Vec<Uuid>>>`) and an `Activation` `gpui_kit::Global` wrapper in `crates/knot/src/app_support.rs`, next to `AwaitingInput`; verify the crate builds (`cargo check -p knot`).
- [ ] 1.2 Register the `Activation` global at app bootstrap alongside `AwaitingInput` (`crates/knot/src/app_bootstrap.rs`) and thread it to wherever `send_message` is dispatched, the way `AwaitingInputQueue` is threaded to the hook/ACP callbacks today; verify with `cargo check -p knot`.

## 2. Trigger activation from a direct send

- [ ] 2.1 In `crates/knot-mcp-tools/src/messaging.rs::send_message`, after `send(...)` returns `Ok`, check the resolved recipient's `activated` flag from the `AgentStore` snapshot already read for lookup; if it is `false`, push the recipient id onto the activation queue. Verify with a unit test: sending to a deactivated recipient's agent id lands in the queue; sending to an already-activated recipient does not.
- [ ] 2.2 Verify a send rejected by any routing rule (cross-workspace, shell agent, companion violation) does not push to the queue - extend the existing rejection tests in `messaging.rs` to assert the queue stays empty.
- [ ] 2.3 Verify `broadcast_message` never pushes to the activation queue, including when an eligible recipient is deactivated - add a test alongside the existing broadcast tests.

## 3. Draining the queue into a running session

- [ ] 3.1 In `crates/knot/src/workspace_window/repaint.rs::panel_needs_repaint`, drain the activation queue each poll (same shape as the existing `waiting` / `drain_panel_prompt` loop): for each queued id that belongs to an agent in this window's workspace, call `ensure_session` (`crates/knot/src/workspace_window/sessions.rs`) and fold whether anything was drained into the function's dirty-check return, matching how `prompt_picked_up` was wired up for the queued-message repaint fix. Verify with `cargo check -p knot`.
- [ ] 3.2 Confirm an id belonging to a different, closed, or non-matching workspace is left in the queue rather than dropped, so whichever window does own that workspace can still pick it up - or document in a code comment why it's safe to drop if a workspace can never legitimately go unclaimed. Verify by reading `ensure_session`'s existing guards and, if a workspace check is added here, exercising it with a test or manual repro.
- [ ] 3.3 Verify end-to-end by manual repro: deactivate an agent, have another agent in the same workspace send it a direct message via the MCP tool, and confirm the sidebar row shows it as running without any user interaction.

## 4. Spec conformance

- [ ] 4.1 Run `openspec validate --strict --change activate-targeted-agent-on-message` and resolve any reported issues.
- [ ] 4.2 Confirm each new/modified scenario in `specs/mcp-messaging/spec.md` and `specs/agent-lifecycle/spec.md` maps to a test written in tasks 2-3; add any missing coverage.
- [ ] 4.3 Run `make lint` and `make test` for the full workspace gate before opening a PR.
