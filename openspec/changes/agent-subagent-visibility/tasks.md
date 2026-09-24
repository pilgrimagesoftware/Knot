# Tasks

## 1. `knot-subagents`: the typed model

- [x] 1.1 Create the crate with a `thiserror`-based `SubagentError` and a crate `Result` alias, registered in the workspace and in `[workspace.dependencies]`; verify `cargo build --workspace` picks it up and `make size-check` still passes
- [x] 1.2 Define `SubagentState` as a closed enum — `Running`, `Finished`, `Failed { reason: Option<String> }` — with `Display`/`FromStr` and no `_ => default` arm at any match site; verify a round-trip test covers every variant and that adding one breaks the match rather than falling through
- [x] 1.3 Define `SubagentKind` so an unstated kind is a distinct value, not an empty string or a substituted default; verify a dispatch with no kind renders as unstated and is not equal to a dispatch naming a kind called "unstated"
- [x] 1.4 Define `Subagent { id: SubagentId, kind: SubagentKind, task: String, started: Instant, state: SubagentState }` with `elapsed(now)` running to completion for a finished record and to `now` for a running one; verify a finished record's elapsed stops advancing and a running one's does not
- [x] 1.5 Define `SubagentEvent::{Dispatched { id, kind, task }, Completed { id, outcome }}` as the single shape both feeds produce; verify the ACP and hook recognizers in group 2 can each be written against it with no feed-specific variant

## 2. `knot-subagents`: recognition, fixtures first

- [x] 2.1 Build fixtures from the adapter's own emit code (`@agentclientprotocol/claude-agent-acp`, `toolCallNotification` and `claudeCodeMetaFromToolUse`) rather than from a captured session - the generator, not one sample of its output: a delegation tool call, a non-delegation tool call, and the delegation's completion update; verify the fixtures are committed as files, each naming the adapter version and the function it mirrors, and that no test constructs the JSON inline
- [x] 2.2 Define `trait Recognizer` taking a tool call's id, raw input and status and answering `Option<SubagentEvent>`; verify a recognizer that returns `None` for every input compiles and is a valid registration, which is what an unsupported agent type is
- [x] 2.3 Implement the Claude Code recognizer against the fixtures, keying on `_meta.claudeCode.subagent` (falling back to `_meta.claudeCode.toolName` being `Task` or `Agent`) and reading the subagent kind and task from `rawInput`'s `subagent_type` and `description`; verify the delegation fixture yields `Dispatched` with the right kind and task, the non-delegation fixture yields `None`, and neither `kind` nor `title` is read anywhere in the implementation
- [x] 2.4 Map the completion fixture's status to `Finished` or `Failed`, retaining the reported reason; verify a failed completion keeps its reason and that an unrecognized status yields `None` rather than `Finished`
- [x] 2.5 Fail closed on partial input: raw input naming a delegation but carrying no task yields `None`, not a record with an empty task; verify a truncated fixture produces no event
- [x] 2.6 Implement the hook recognizer over the posted payload shape from the `agent-hooks` delta; verify a dispatch payload missing its identifier and one missing its task each yield `None` (the 400 is the route's job, in group 5), and that a dispatch with no kind yields `Dispatched` with the kind unstated

## 3. `knot-subagents`: the registry

- [x] 3.1 Implement `SubagentRegistry` keyed by agent `Uuid`, applying a `SubagentEvent` to create or update one record; verify a `Completed` for an id with no dispatch is ignored and creates nothing
- [x] 3.2 Order each agent's records running-first, then longest-running first within each group, as `agent-processes` requires; verify a mixed set of two running and one finished sorts as the spec's scenario states
- [x] 3.3 Implement `clear(agent)` for a turn ending, an agent stopping and a session ending; verify a cleared agent holds no records and that clearing one agent leaves another's intact
- [x] 3.4 Cap retained finished and failed records per agent at a `consts.rs` value, discarding oldest first and never discarding a running record; verify a cap of N plus one finished record drops the oldest, and that N running records plus one more retains all of them
- [x] 3.5 Add `take_changed()` clearing a dirty flag set by every mutation; verify a mutation sets it, a second read returns false, and — the breakage `.claude/rules/rust-structure.md` names four times — that a test fails if any caller other than the repaint drain calls it

## 4. ACP carries the raw input it already receives

- [x] 4.1 Add the raw input and the `_meta` envelope to `SessionUpdate::ToolCallStart` and `ToolCallUpdate` as `Option`s, parsed from the wire in `from_params`; verify a fixture with neither decodes to `None` for both and one with `{}` decodes to an empty object, distinguishably
- [x] 4.2 Confirm every existing construction site and match arm still compiles and that no existing `knot-acp` test changes its expectation; verify `cargo test -p knot-acp` passes unchanged
- [x] 4.3 Carry the raw input and `_meta` onto `ToolCallCard` through `panel_state/fold.rs`, and verify an update carrying only a status change leaves both intact rather than blanking them

## 5. The two feeds write the one registry

- [x] 5.1 Hold the registry as `Arc<Mutex<SubagentRegistry>>` shared by `McpToolCatalog` and the workspace window; verify both read the same instance in a test that writes through one and reads through the other
- [x] 5.2 Fold ACP tool calls into the registry from `panel_session.rs`'s apply path, off the main thread, selecting the recognizer by the agent's type; verify a session fed the group 2 fixtures leaves exactly one running record, then one finished one
- [x] 5.3 Clear an agent's records on `SessionUpdate::TurnEnd` and on `SessionEvent::Ended`; verify a turn ending empties the agent's list and leaves other agents alone
- [x] 5.4 Accept the subagent hook events on the existing status route, keyed on the `hook` field so the activity status is untouched; verify a dispatch posted while the agent is Working leaves it Working and raises no desktop notification
- [x] 5.5 Return 400 for a subagent event with no identifier and for a dispatch with no task, and success-with-no-change for a completion naming an unknown identifier; verify all three against the `agent-hooks` delta's scenarios
- [x] 5.6 Clear an agent's records when a Claude hook reports the turn ended (status `idle`) and when the agent is deactivated or removed; verify a deactivated agent holds no records

## 6. The roster says which types can report

- [x] 6.1 Add a `SubagentReporting` column to `AgentTypeInfo` as a closed enum — `None`, `ToolCalls`, `Hooks`, `Either` — with a `can_report(view_mode)` resolving it against the agent's view mode; verify a `ToolCalls` type resolves to reporting in Panel mode and not in Terminal mode
- [x] 6.2 Populate it: Claude as `ToolCalls` (not `Either` — the hook emitter is a plugin outside this repo, per design.md), every other type and `shell` as `None`; verify a Terminal-mode Claude agent resolves to unavailable rather than to "dispatched none"
- [x] 6.3 Extend the roster's existing completeness test to cover the new column; verify the test fails when a row's value is removed

## 7. Splitting the pane before the second row kind lands

- [x] 7.1 Split `render/processes_pane.rs` into a `processes_pane/` directory — `mod.rs` declaring only, siblings for the header, the process rows, and the empty states — with no behavior change; verify `make` passes and the existing `processes_pane/tests.rs` is unchanged apart from its module path
- [x] 7.2 Verify `make size-check` passes and that `mod.rs` holds no implementation, per the repo's "`mod.rs` declares; it does not implement" rule

## 8. The section renders two groups

- [x] 8.1 Give `ProcessSection` the agent's subagent list beside its process snapshot, read from the registry rather than copied into it; verify collapsing the section discards neither
- [x] 8.2 Render the two labelled groups in order — subagents, then processes — with no indentation of one under the other; verify a snapshot test shows both labels and that no subagent row is indented beneath another row
- [x] 8.3 Render a subagent row as kind, task, elapsed time and state, with the task on one line truncating to an ellipsis and no process identifier anywhere on the row; verify a 2 KB task renders at a one-line row's height and that the row contains no PID
- [x] 8.4 Format a subagent's elapsed time through the existing `processes.runtime_*` keys; verify a four-minute record renders through the minutes shape and that no new duration formatter is introduced
- [x] 8.5 Offer a copy-task action on a subagent row and no terminate, process-identifier or process-viewer action; verify the copy places the untruncated task on the clipboard and that no code path can reach `terminate` from a subagent row
- [x] 8.6 Omit the subagents group entirely for an agent whose type cannot report; verify a shell agent's expanded section shows the processes group alone and states nothing about subagents

## 9. The header summary

- [ ] 9.1 Extend the collapsed summary to name distinct subagent kinds before distinct process names, deduplicating each; verify three `code-review` subagents name the kind once and that the remainder counts the other two
- [ ] 9.2 Count subagents and processes separately in the expanded summary; verify two subagents and three descendants read as two and three, not as five
- [ ] 9.3 Make a stopped agent read as nothing running whatever the registry and the last sample hold; verify an agent that stops while holding both still summarizes as nothing running
- [ ] 9.4 Keep the unknown marker tied to the first process sample only; verify an agent with a recorded subagent and no completed sample still reads as unknown, and that a subagent list alone is never reported as unknown

## 10. Empty states and localization

- [x] 10.1 Extend the pane's empty-state vocabulary to cover the subagents group — the agent is not running, or it dispatched none — alongside the existing process cases, as a closed enum with no default arm; verify each of the delta's empty-state scenarios maps to a distinct variant
- [x] 10.2 Keep a failed process sample off the subagents group: the process rows keep their last successful sample and their failure notice, the subagent rows carry neither; verify a failed sample leaves the subagent rows unchanged
- [ ] 10.3 Add the new keys under `processes.` in `crates/knot-core/locales/en.yml` — group labels, subagent state labels, the subagent counts, the copy-task action and its confirmation, and the two new empty states; verify tests assert the keys resolve and never the English copy, and touch `knot-core` after editing the catalog so the l10n tests do not run against a stale artifact
- [ ] 10.4 Show a subagent's kind and task verbatim, never through the localization lookup; verify a kind that collides with a localization key renders as the agent reported it

## 11. Reaching a frame, and the gate

- [ ] 11.1 Add the single branch in `repaint_poll_tick` that takes the registry's changed flag and contributes to the `cx.notify()` condition; verify a subagent completing while the agent's pane is shown repaints without any other event, and that no other caller takes the flag
- [ ] 11.2 Verify no registry read, recognizer call or clock-driven recomputation is reachable from a `render` function, the way `diff_stats.rs` guards the same rule
- [ ] 11.3 Verify a subagent dispatched while another agent's pane is shown is recorded and appears when that agent's pane is next shown — the spec's scenario that separates recording from sampling
- [ ] 11.4 Run `make` and verify the whole gate passes: `fmt-check`, `size-check`, `lint` with `-D warnings`, `test` and `build`
