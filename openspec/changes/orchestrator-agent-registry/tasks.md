# Tasks

## 1. Registry fields on the durable records

- [x] 1.1 Add `CostTier { Low, Medium, High }` to `knot-core` with `Display`,
  `FromStr`, `Default` returning `Medium`, and serde; verify a unit test
  round-trips each variant through string and JSON and rejects an unknown
  value with a typed error rather than a default
- [x] 1.2 Add `description: String`, `capabilities: BTreeSet<String>` and
  `cost_tier: CostTier` to `SavedAgent` and `BenchAgent` in
  `crates/knot-core/src/settings/records.rs`, all `#[serde(default)]`;
  verify the existing `crates/knot-core/tests/fixtures/settings_swift_shape.json`
  fixture still loads and the three fields read back as empty / empty /
  `Medium`
- [x] 1.3 Normalize capability tags on write (trim, lowercase, drop empties);
  verify a test that `[" Rust ", "rust", ""]` stores as the single tag
  `rust`
- [x] 1.4 Mirror the three fields onto `knot_agents::Agent` as durable
  fields and carry them through `convert::from_saved` and the persist path;
  verify the existing reload test still shows runtime fields resetting while
  the three new ones survive a round trip
- [x] 1.5 Carry the three fields through save-to-bench and bench deployment;
  verify a test that an agent tagged `testing` at `Low` saved to the bench
  and redeployed comes back with the same tag and tier
  (`agent-registry` - "Registry metadata survives the bench round trip")

## 2. The registry projection

- [x] 2.1 Add a registry view that projects live agents plus bench templates
  into candidates carrying id, description, tags, reachable tools, cost tier
  and status, computed from `agents_snapshot()` and `Settings::bench_agents`
  with no new stored collection; verify a test that a bench entry projects
  with status `template` and a live agent projects with its automatic state
- [x] 2.2 Implement tag matching (all requested tags must be present) and
  the ranking order - idle live, then other live, then templates; within
  each, ascending cost tier, then name; verify tests for the
  cheapest-idle-first, busy-outranked-by-idle, and all-tags-must-match
  scenarios in `agent-registry`
- [x] 2.3 Apply the existing visibility rules (caller's workspace; companions
  only if owned by the caller) to registry results; verify a test that an
  unowned companion carrying a queried tag is absent from the candidates
- [x] 2.4 Verify an empty result for an unmatched tag is a success, not an
  error, with a test asserting the candidate list is empty and no error is
  raised

## 3. The `knot-tasks` crate

- [x] 3.1 Create the `knot-tasks` crate in the workspace (no async runtime,
  no UI or MCP dependency) with `Task`, `TaskState`, `Assignee` and
  `TaskGraph`; verify `cargo build -p knot-tasks` succeeds and the crate's
  only dependencies are `thiserror`, `uuid`, `serde` and `knot-core`.
  `knot-core` was not in the original plan: it is taken for `Capabilities`
  alone, because a second tag representation here would reintroduce the
  normalization mismatch that type exists to prevent. It brings no runtime,
  no UI and no MCP types, so the crate is still testable on its own
- [x] 3.2 Implement commit-time validation - dangling dependency, self
  dependency, cycle, task limit - returning a typed error naming the
  offending task or edge; verify tests for each of the three rejection
  scenarios in `task-graph` plus one asserting a rejected commit leaves a
  previously committed graph untouched
- [x] 3.3 Implement the state machine: initial `ready`/`pending` assignment,
  the readiness rule, the `dispatched` transitions, and transitive
  `blocked` propagation on failure; verify tests for the last-dependency,
  failure-blocks-downstream and blocked-does-not-recover scenarios
- [x] 3.4 Implement the dependency gate as a query that returns either the
  assignee to dispatch to or the unmet dependency ids, without performing
  any delivery; verify a test that gating a task with one outstanding
  dependency returns that id
- [x] 3.5 Implement re-plan carry-over: tasks present in both graphs keep
  their state, tasks only in the old graph are dropped, and no dispatched
  task is cancelled or re-sent; verify tests for the dispatched-survives and
  done-not-re-run scenarios
- [x] 3.6 Add the task limit and any ranking constants to the crate's
  `consts.rs`; verify `make size-check` passes and no new `.rs` file exceeds
  700 lines

## 4. Split `knot-mcp-tools` before it outgrows the cap

- [ ] 4.1 Move the tool catalogue and the `ToolCatalog::call` dispatch match
  out of `crates/knot-mcp-tools/src/lib.rs` into a new `catalog.rs`; verify
  `make size-check`, `make lint` and `make test` all pass with the existing
  thirteen tools unchanged
- [ ] 4.2 Confirm the new module is declared and reachable (`mod` in
  `lib.rs`, re-exported where the old paths were used); verify the existing
  `crates/knot-mcp/tests/http.rs` suite passes untouched

## 5. Registry MCP surface

- [ ] 5.1 Extend the `list-agents` payload with description, capability tags,
  reachable tools and cost tier; verify tests for the registry-fields and
  undescribed-agent scenarios in the `mcp-tools` delta
- [ ] 5.2 Add the `describe-agents` handler with `capabilities` and
  `includeTemplates` arguments; verify tests for query-by-tag,
  templates-excluded and no-match-is-not-an-error
- [ ] 5.3 Register `describe-agents` in the catalogue with a typed JSON input
  schema; verify the `tools/list` test asserts the new tool count and that
  every tool still declares an object schema

## 6. Task MCP surface

- [ ] 6.1 Add a `tasks/` module beside `agents/` holding the four task tool
  handlers; verify `make size-check` passes and each handler file stays well
  under the cap
- [ ] 6.2 Implement `plan-tasks`: parse the task array, commit through
  `knot-tasks`, return each id and resulting state, and surface a validation
  failure as `isError` true naming the offending edge; verify tests for the
  valid-plan and cyclic-plan scenarios
- [ ] 6.3 Implement `dispatch-task`: check the gate, resolve a tag assignee
  against the registry, deliver through the existing messaging path, and
  mark `dispatched` only on successful delivery; verify tests for
  ready-task-delivered, unmet-dependency-reported,
  rejected-delivery-leaves-task-ready, tag-resolves-at-dispatch and
  no-candidate-for-tag
- [ ] 6.4 Implement `complete-task` returning the ids that became `ready` and
  `blocked`, and `task-status` returning the caller's graph or an empty list;
  verify tests for completion-readies-dependent, failure-blocks-dependents
  and status-before-planning-is-empty
- [ ] 6.5 Register all four tools in the catalogue with typed input schemas;
  verify the `tools/list` test asserts all eighteen tools are present
- [ ] 6.6 Verify the unplanned fan-out scenario end to end: dispatching with
  no committed graph is refused with a message telling the caller to plan
  first (`task-graph` - "Nontrivial work requires a committed plan")

## 7. Editing registry metadata

- [ ] 7.1 Add description, capability-tag and cost-tier controls to the agent
  editor, with all copy through `knot_core::l10n::t` and keys added to
  `en.yml`; verify the l10n test asserts every new key resolves (touch
  `knot-core` first so the catalogue is not read from a stale artifact)
- [ ] 7.2 Add the same three fields to the bench entry path; verify editing
  them persists through a settings round trip
- [ ] 7.3 Ensure editing any of the three does not restart the agent; verify
  a test that adding a tag to a Working agent leaves its session intact
  (`agent-lifecycle` - "Re-tagging a working agent does not interrupt it")

## 8. Showing the plan

- [ ] 8.1 Render `Agent::mermaid_source` in the agent panel, honouring
  `mermaid_title`; verify the existing `view-mermaid` tool now produces a
  visible diagram, since nothing under `crates/knot/` consumed that state
  before
- [ ] 8.2 Emit mermaid text for a committed graph, with each task's state
  shown, and write it through the same panel-state path `view-mermaid` uses;
  verify a snapshot test of the generated mermaid for a four-task graph
- [ ] 8.3 Re-emit the diagram on every task state change; verify a test that
  reporting a task `done` updates the stored mermaid source
  (`task-graph` - "The diagram tracks state")

## 9. Shipped Orchestrator persona

- [ ] 9.1 Add an "Orchestrator" system persona to the shipped defaults in
  `crates/knot-core/src/consts.rs` with a stable id, instructing the query →
  plan → dispatch method and naming no teammate; verify the existing
  install-on-startup test picks it up and restore-defaults restores it
  unchanged
- [ ] 9.2 Confirm the persona text contains no agent names or counts; verify
  by inspection against `agent-registry` - "A roster is obtained by query,
  never stored as text"

## 10. Verification

- [ ] 10.1 Run `make` and verify the whole gate passes: `fmt-check`,
  `size-check`, `lint`, `test`, `build`
- [ ] 10.2 Verify no crate-wide `allow` was added and every per-item allow
  carries a reason comment, per `.claude/rules/rust-structure.md`
- [ ] 10.3 Verify a pre-existing `settings.json` from before this change
  loads with no error and every agent reads back with an empty description,
  no tags, and cost tier `medium`
