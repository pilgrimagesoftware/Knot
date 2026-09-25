# Design

## Context

See proposal.md - Why. The pieces this change builds on:

- **Initialization prompt.** `knot_agent_launch::acp_registration_prompt`
  builds the first turn of a fresh ACP session; `panel_session::connect_into`
  records it, publishes the `Ready` slot, then awaits `session.prompt` on it.
  Only `shell`-type agents run in a terminal; see "No terminal delivery".
- **Prompt queue.** `workspace_window/prompt_queue.rs` holds per-agent
  `QueuedPanelPrompt`s tagged with a `PromptOrigin` (`User`, `InboxNudge`).
  `drain_panel_prompt` delivers the head once the session is `Ready` with no
  `turn_active` and no pending permission, and is driven from
  `repaint_poll_tick` for every agent, selected or not.
- **Bench.** `BenchAgent` and `add_bench_agent` in `knot-core`,
  `AgentStore::deploy_bench` in `knot-agents`, Save to Bench in
  `menus/agent_row.rs`, deployment reachable only through `create-agent`.
- **Settings.** One store with a document per durable collection, a
  `persist_*` per document, and `settings_global::write_persisting` for a
  write from the UI. Tabs are the closed `SettingsTab` enum.

## Goals / Non-Goals

**Goals**

- Reuse the existing queue for the ACP startup prompt rather than adding a
  second delivery path.
- Keep the initialization prompt's builder untouched; the startup prompt is
  resolved and delivered separately.
- Add the prompt library as one more durable document, following the
  personas pattern end to end (record, document, persist, Settings tab).

**Non-Goals**

- No change to how `create-agent` deploys beyond `deploy_bench` copying one
  more field.
- No shared "prompt picker" component beyond what the agent editor and the
  Bench tab both need; if they converge, extracting it is a follow-up.

## Decisions

### `StartupPrompt` is an enum, and absence is `Option`

```rust
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum StartupPrompt { Library(Uuid), Custom(String) }
```

`SavedAgent` and `BenchAgent` gain `#[serde(default)] startup_prompt:
Option<StartupPrompt>`. A closed vocabulary is an enum per the project
conventions; "none" is `None` rather than a third variant so a missing field
decodes to it for free.

*Alternatives.* A library id only: forces a library entry for every one-off
prompt. Text only, with the library as a fill-in source: editing a library
prompt would not reach the agents that use it, which is most of the point of
a library.

### A dangling reference resolves to nothing and is kept

Removing a prompt does not rewrite the agents or bench documents. Clearing
references would make a library delete write three documents, against
"A write touches only the document it belongs to", and would hide from the
editor why an agent stopped getting its prompt. Resolution lives in
`knot-agent-launch` (`resolve_startup_prompt(&Option<StartupPrompt>,
&[Prompt]) -> Option<String>`), next to the registration builder it follows.

### Variables: a closed enum, a context struct, one pure expander

`knot-agent-launch` gains `PromptVariable` (`AgentName`, `AgentId`,
`AgentType`, `Folder`, `FolderName`, `Workspace`, `Branch`, `Date`) with
`Display`/`FromStr` over the dotted names - a closed vocabulary, per the
project conventions - and a `PromptContext` holding every value already
resolved:

```rust
pub struct PromptContext { agent_name, agent_id, agent_type, folder,
                           workspace: Option<String>, branch: Option<String>,
                           date: String }
pub fn expand(text: &str, ctx: &PromptContext) -> String;
pub fn unknown_variables(text: &str) -> Vec<String>;
```

`expand` is a single left-to-right scan: `\{{` emits `{{` and skips; `{{`
up to the next `}}` with a trimmed name that parses as a `PromptVariable`
emits the value; anything else is copied through. Values are appended to the
output and never rescanned, which is what makes double expansion impossible
rather than merely unlikely. `unknown_variables` is the same scan collecting
names that fail to parse, used by both editors for their warnings.

Keeping the expander pure over a pre-resolved context puts every piece of
I/O in the caller, where each call site already knows which thread it is on.

*Alternatives.* A template engine (`minijinja`, `handlebars`): a new
dependency, a syntax far larger than eight names, and error modes (a
malformed tag failing the whole render) that the unknown-name rule exists to
avoid. Resolving variables inside `expand` with callbacks: hides the `git`
read inside a function that looks pure, which is how I/O ends up on the
render path.

### Where the context is built

- **Startup prompt, ACP.** `ensure_panel_session` resolves the library
  reference to raw text on the UI thread (a settings read, no I/O) and
  passes it with the context's cheap fields into the runtime task that
  already runs the connect. That task reads `branch`, expands, and stores
  the result on the session handle before publishing `Ready`. See "ACP
  delivery" for how it reaches the queue.
- **Slash insertion.** The lookup captures the token's range and the buffer
  revision, then expands on `spawn_blocking`. The result lands in a
  per-window slot drained from `repaint_poll_tick`'s `if` chain; it is
  applied only if the composer's revision still matches, otherwise dropped.
  The slot is drained with a non-clearing check first so a discarded frame
  cannot swallow it (the failure named in `.claude/rules/rust-structure.md`).
- **`branch`.** `knot_git::Repository::current_branch`; on `None` (detached
  HEAD) the short `HEAD` hash; on a not-a-repository error, empty.
- **`date`.** `time::OffsetDateTime::now_local()`, falling back to UTC when
  the local offset cannot be determined (the `time` crate refuses it in some
  multi-threaded contexts); `time` is already a workspace dependency, and
  its `local-offset` feature is added.
- **`workspace`.** The name of the workspace containing the agent, read from
  the settings surface when the context is built.

### Prompts are their own document

`prompts.json` (`PROMPTS_FILE` in `knot-core/src/consts.rs`) beside
`personas.json`, with `persist_prompts`. Not folded into personas: a persona
is standing identity sent inside the initialization prompt; a library prompt
is a task sent as its own turn, and they are edited in different places.

### ACP delivery: queue at connect, hold the pump on the registration turn

When `ensure_panel_session` builds a registration prompt (a fresh session),
the expanded startup prompt travels on the session handle (see "Where the
context is built"). `queue_startup_prompts`, run first thing in
`deliver_waiting_prompts`, takes each `Ready` handle's startup prompt and
pushes it onto that agent's queue as `PromptOrigin::Startup`; the existing
pump then delivers it when the registration turn ends. Taking it there, not
in a callback, keeps the handoff on the path `repaint_poll_tick` already
polls for every agent, and its result feeds the tick's repaint flag.

There is a window in which the slot is `Ready` but the registration
`session.prompt` has not yet flipped `turn_active`, and the pump could send
the startup prompt first. It cannot: `connect_into` records the
registration message before publishing `Ready`, and `push_user_message`
already sets `turn_active`, so the pump sees a turn in flight from the first
frame it can see the session at all. A test drives a fake adapter whose first turn never
answers and asserts the startup prompt stays queued.

If the registration turn fails, the startup prompt is still delivered; if it
fails too, it holds in the queue as `failed`, which is the existing
behaviour for any queued prompt and gives the user retry or delete.

*Alternatives.* Chaining a second `session.prompt` inside `connect_into`:
invisible to the queue, so the user could not edit or delete it, and it would
bypass the permission gate the pump already honours. Enqueuing only after the
registration turn resolves: needs a new signal from the runtime task back to
the window, and the prompt would not be visible while it waits.

### No terminal delivery

The proposal first had a terminal path too: a one-shot in `knot-activity`
typing the prompt at the idle after registration. Implementation showed it
could never fire. `plan_launch` gives every non-shell type its ACP adapter,
with no terminal fallback, so only `shell`-type agents run in a terminal -
and a `shell`-type agent carries no startup prompt. The path, and the
one-line flattening it needed, were dropped rather than built as unwired
code.

### Benching is a store operation that writes first

`bench_agent` in the `knot` crate reuses Save to Bench's entry construction
(extracted into one `bench_entry_for(&Agent) -> BenchAgent` so the two items
cannot drift), writes it through `write_persisting`, and only on `Ok`
removes the agent through the existing removal path. Removing after a failed
write would lose the agent's configuration entirely.

### Bench deployment from the sidebar

New from Bench is a submenu on the background menu built from the settings
surface's bench list at menu-build time, so an entry saved from another
window appears without a snapshot refresh (the staleness `sidebar_layout.rs`
already documents). Deployment calls `AgentStore::deploy_bench` with the
sidebar's workspace; a `None` result removes the entry and posts a
notification naming it.

### The New Agent split button and bench popover

`new_agent_button` in `workspace_window/render/mod.rs` becomes two adjacent
controls in one row: the existing ghost `Button` (unchanged behaviour) and a
chevron `Button` that toggles a gpui-kit `Popover` anchored to the row. The
popover's content is a small view, `workspace_window/bench_popover.rs`,
built on open from `settings_global::read(cx).bench_agents` - read on open,
not cached, which is both what the spec requires and what keeps it out of
the repaint chain.

- Rows are hover-tracked with `bench_popover_hovered: Option<Uuid>` on the
  window; the remove control is always rendered, muted until its row is
  hovered, so keyboard users can reach it (the Swift view shows it on hover
  only). Tracking per-row focus to reveal it would need a focus handle per
  entry for no gain over always drawing it.
- Deploying and New from Bench share one `deploy_bench_entry(entry, cx)` on
  `WorkspaceWindow`, so stale-entry pruning and its notification exist once.
- Removal confirms through the workspace menus' `confirm_then`, then
  `write_persisting(|s| s.remove_bench_agent(id))`; the popover reads the
  bench on each draw, so it stays open on the updated list.
- The 300px scroll cap and the 260px popover width are layout numbers and
  stay inline; nothing here is a decision for `consts.rs`.

*Alternative.* A native `NSMenu` dropdown like the context menus: cannot
show the avatar-and-two-line rows or an inline remove control, which are
the parts of the Swift view worth porting.

### Slash lookup source

A third registry source alongside built-in commands and skills, reading the
library from the settings global at lookup time (so a library change reaches
an open composer without wiring a new cache into `repaint_poll_tick`). Its
entries carry a kind so the popup can mark them, and insertion switches on
that kind: token replacement for commands and skills, text expansion for
prompts.

### Settings tabs

`SettingsTab` gains `Prompts` and `Bench` after `Personas`. The Prompts tab
and its editor follow `persona_editor.rs`; the Bench tab's editor edits name
and startup prompt only.

## Risks / Trade-offs

- [An expansion that runs `git` on the UI thread] → The expander is pure;
  every context that needs `branch` is built on a runtime or blocking
  thread, and the slash-insertion result is revision-checked before it is
  applied.
- [`now_local` fails and `{{date}}` is a day off near midnight] → Accepted;
  UTC fallback is documented in code.
- [The queued startup prompt races the registration turn] → `turn_active`
  set before `Ready`; covered by a stalled-adapter test.
- [Benching loses the conversation] → Confirmation says so; Save to Bench
  remains for copying without closing.
- [`knot-core` records and settings files near the 700-line limit] → New
  types go in their own file (`settings/prompts.rs`) if `records.rs` would
  exceed it; check with `make size-check` before committing.

## Migration Plan

Additive only. New fields are `#[serde(default)]`, so existing agents and
bench entries load with no startup prompt, and a store with no
`prompts.json` loads an empty library and writes none until a prompt is
added. Rollback: an older build ignores the unknown field and document.
