# Tasks

## 1. The tracker map

- [ ] 1.1 Declare `panel_trackers: BTreeMap<Uuid, knot_activity::Tracker>` on `WorkspaceWindow` in `workspace_window/window.rs`, beside `panel_prompt_queues`, and initialise it in `workspace_window/open.rs`; verify with `make build`
- [ ] 1.2 Drop the agent's entry in the per-agent teardown in `workspace_window/sessions.rs` alongside `panel_prompt_queues.remove(&id)`; verify by asserting the map is empty after removing the only agent that had a panel session
- [ ] 1.3 Add `knot-activity` to `crates/knot/Cargo.toml` if it is not already a dependency, and verify `cargo build -p knot` resolves it

## 2. Building a Panel tracker

- [ ] 2.1 Add a `panel_tracker_for(&mut self, id: Uuid, agent_type: &str) -> bool` helper in `workspace_window/repaint.rs` (or a sibling module if `repaint.rs` nears the 700-line cap) that returns an existing tracker, or spawns one under `self.runtime.enter()` with `tracking_for(agent_type, ViewMode::Panel)` and bails on `Handle::try_current().is_err()` as `knot-mcp-tools::tracker_for` does; verify it returns `ACP_UPDATES` tracking with a unit test on the preset
- [ ] 2.2 Give the tracker an `EventSink` whose `on_status` writes `store.set_state(id, event.status)` and whose `on_awaiting_input` pushes `(id, message)` onto the `AwaitingInputQueue` global, mirroring `knot-mcp-tools/src/lib.rs:155`; verify the sink writes the store by driving a status through a spawned tracker in a test
- [ ] 2.3 Confirm the `AwaitingInputQueue` reaches `WorkspaceWindow` (it is already built in `app_bootstrap.rs:515` and set as the `AwaitingInput` global); read it from the global rather than threading a new field, and verify with `make build`

## 3. Routing the poll through the tracker

- [ ] 3.1 In `sync_panel_agent_states`, carry the pending permission's `tool_call_title` alongside the derived state so the message travels with the transition; verify with a unit test over the derivation that a pending request yields `(Input, Some(title))`
- [ ] 3.2 Replace the `store.set_state(id, state)` call with `apply_acp_status(state, message)` on the agent's tracker, gated on the existing "differs from what the store holds" comparison so a still-pending request reports once; verify with a test that N ticks over one unchanged pending permission produce one `AwaitingInput` effect
- [ ] 3.3 Keep the direct `store.set_state` as the fallback branch when no tracker could be spawned, matching `knot-mcp-tools`; verify with a test that the fallback still moves the dot and emits no effects
- [ ] 3.4 Confirm `panel_states_moved` still reports honestly now that the write is asynchronous — it must stay true on the tick the transition is reported, or the frame that shows the new dot is skipped; verify by asserting the returned bool on a transition tick

## 4. Behavior tests against the spec

- [ ] 4.1 Test: a Panel-mode agent with a pending permission request enters `AgentState::Input` and produces an `AwaitingInput` effect carrying the tool call title — `activity-detection`'s "Permission request raises a notification"
- [ ] 4.2 Test: a turn ending with no pending permission goes through `mark_idle` and produces `CheckMessages` — `activity-detection`'s "A finished turn is a delivery opportunity"
- [ ] 4.3 Test: a permission request with `tool_call_title: None` carries no message, so the notification falls back to its default body — `desktop-notifications`' "Notification uses default body without a message"
- [ ] 4.4 Test: repeated ticks over one unchanged pending permission raise one notification — `desktop-notifications`' "A still-pending permission request is not re-notified"

## 5. Gate and wrap-up

- [ ] 5.1 Check no `.rs` file crossed 700 lines and split by concern if one did; verify with `make size-check`
- [ ] 5.2 Run `make` (fmt-check, size-check, lint, test, build) and verify the whole gate passes
- [ ] 5.3 Confirm `Tracker::apply_acp_status` now has a production caller and carries no stale `UNWIRED`-style comment; verify by grepping the workspace for the call site
- [ ] 5.4 Run `openspec validate acp-status-through-tracker --strict` and verify it passes
