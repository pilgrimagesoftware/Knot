# Design

## Context

`render_message` (`panel_view/message.rs`) is called from the panel view,
which renders inside `WorkspaceWindow` — the same window that owns
`key_hints: KeyHintHold` (`workspace_window/key_hints.rs`) and already wires
`ModifiersChangedEvent`/`KeyDownEvent`/activation into it for the sidebar's
own ⌘-hold shortcut badges. `render_message` currently takes a `Message<'a>`
struct (`state`, `index`, `is_last`, `style`, `list`) with no access to that
hold state. See proposal.md for why timestamps should move to the same
disclosure model.

## Goals / Non-Goals

**Goals:**
- One ⌘-hold signal drives both the sidebar's key hints and the panel's
  timestamps, so there is a single hold/delay/clear state machine in the
  window, not two.
- Timestamps mount with zero layout cost when hidden (today's `.children(...)`
  `Option` pattern already does this — keep it, just change what feeds the
  `Option`).

**Non-Goals:**
- No change to `KeyHintHold`'s delay, clear, or activation-loss semantics —
  this change is a second consumer of that state, not a redesign of it.
- No new tooltip or formatting behavior for the timestamp itself
  (`relative_timestamp`/`absolute_timestamp` are unchanged).

## Decisions

**Reuse `WorkspaceWindow::key_hints` rather than a second hold tracker.**
A `KeyHintHold` per surface (one for the sidebar, one for the panel) would
double the `ModifiersChangedEvent`/`KeyDownEvent`/activation wiring for no
behavioral difference — both are "⌘ held while this window is key, after a
delay, until a key interrupts or the window deactivates." `Message<'a>` gains
a `cmd_held: bool` field, set by the caller from
`workspace.key_hints.shown().is_some()`, so `message.rs` stays free of
`WorkspaceWindow` as a dependency and only needs a bool.

**Alternative considered:** give `PanelState` its own hold flag, updated by a
new panel-scoped modifier handler. Rejected — it would fire a second timer on
every ⌘ press alongside the sidebar's, and the two could desync (e.g. a
rebinding mid-hold invalidates one `generation` but not the other, per
`key_hints.rs`'s own doc comment about rebinding not landing mid-hold).

**`cmd_held` as a plain bool, not `Option<&SidebarKeyHints>`.** The panel
does not need the sidebar's formatted shortcut labels — only whether a hold
is currently showing. Passing the bool keeps `message.rs` from importing
`SidebarKeyHints`.

## Risks / Trade-offs

- [Risk] Coupling panel timestamp visibility to the sidebar's hold state
  means a future change to sidebar-only hold behavior (e.g. scoping it to
  just the sidebar having focus) silently changes panel timestamps too. →
  Mitigation: the ADDED requirement's scenarios pin the panel's behavior
  independently, so a future change to `KeyHintHold` that breaks panel
  timestamps fails this spec's tests, not just the sidebar's.
- [Risk] This change's branch must start from PR #579's branch, not
  `develop`, since `relative_timestamp`/`render_timestamp` don't exist on
  `develop` yet. → Mitigation: noted in proposal.md's Impact section; verify
  `git branch --show-current` and the merge-base before opening a worktree.
