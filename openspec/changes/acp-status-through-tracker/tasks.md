# Tasks

## 1. The tracker map

- [x] 1.1 Declare `panel_trackers: BTreeMap<Uuid, knot_activity::Tracker>` on `WorkspaceWindow` in `workspace_window/window.rs`, beside `panel_prompt_queues`, and initialise it in `workspace_window/open.rs`; verified by `make build`
- [x] 1.2 Drop the agent's entry in the per-agent teardown in `workspace_window/sessions.rs` alongside `panel_prompt_queues.remove(&id)`; verified by reading the teardown, which now removes the tracker with every other per-agent panel map
- [x] 1.3 `knot-activity` was already a dependency of `crates/knot` (`Cargo.toml:19`); verified by `cargo build -p knot`

## 2. Building a Panel tracker

- [x] 2.1 Add `WorkspaceWindow::ensure_panel_tracker` in the new `workspace_window/panel_activity` module: returns an existing tracker, or spawns one under `self.runtime.enter()` with `tracking_for(agent_type, ViewMode::Panel)`, bailing on `Handle::try_current().is_err()` as `knot-mcp-tools::tracker_for` does; verified by `make build` and `make lint`
- [x] 2.2 Give the tracker an `EventSink` whose `on_status` writes `store.set_state(id, event.status)` and whose `on_awaiting_input` pushes `(id, message)` onto the `AwaitingInputQueue` global, mirroring `knot-mcp-tools/src/lib.rs:155`
- [x] 2.3 Read the `AwaitingInputQueue` from the `AwaitingInput` global rather than threading a new field through the window; verified by `make build`

## 3. Routing the poll through the tracker

- [x] 3.1 Extract the derivation as the pure `panel_activity::acp_status(&PanelState) -> (AgentState, Option<String>)`, carrying the pending permission's `tool_call_title` as the attention message; verified by five unit tests over the mapping
- [x] 3.2 Extract the level-to-edge gate as the pure `panel_activity::transitions`, and call `apply_acp_status` only for agents whose status differs from the store's; verified by `one_pending_permission_across_many_ticks_reports_once`, which drives thirty ticks over one unchanged pending permission and asserts a single report
- [x] 3.3 Keep the direct `store.set_state` as the fallback branch when no tracker could be spawned, matching `knot-mcp-tools`
- [x] 3.4 `sync_panel_agent_states` returns true on any tick that reports a transition, so the frame showing the new dot is not skipped; verified by reading the rewritten function, whose `moved` list is exactly the reported transitions

## 4. Behavior tests against the spec

- [x] 4.1 Test: a pending permission maps to `AgentState::Input` carrying the tool call title — `activity-detection`'s "Permission request raises a notification"
- [x] 4.2 Test: a finished turn with nothing pending maps to `Idle`, which is the status that routes through `mark_idle` — `activity-detection`'s "A finished turn is a delivery opportunity"
- [x] 4.3 Test: a permission request with `tool_call_title: None` carries no message — `desktop-notifications`' "Notification uses default body without a message"
- [x] 4.4 Test: repeated ticks over one unchanged pending permission report once — `desktop-notifications`' "A still-pending permission request is not re-notified"

## 5. Gate and wrap-up

- [x] 5.1 No `.rs` file crossed 700 lines; verified by `make size-check` (526 files, none over)
- [x] 5.2 Run the whole gate; verified by `make` — fmt-check, size-check, lint, test and build all pass
- [x] 5.3 `Tracker::apply_acp_status` now has a production caller in `sync_panel_agent_states`; verified by grepping the workspace for the call site
- [x] 5.4 Run `openspec validate acp-status-through-tracker --strict`; verified passing

## 6. Follow-ups, not part of this change

- [ ] 6.1 The sink and the no-tracker fallback are covered by reading, not by a test: both need a live `WorkspaceWindow`, which needs GPUI and a window handle. Reaching them wants a seam that does not exist yet — worth its own change rather than a fixture bolted on here
