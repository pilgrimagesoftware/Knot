# Proposal

## Why

`WorkspaceWindow` holds `knot_core::Settings` by value, assigned when the
window is built and never reassigned. A preference changed in the settings
window is written to disk correctly and never reaches a workspace window that
is already open.

This is the read half of issue #238. It is not theoretical: it is what makes
`collapsed-tool-call-summary` fail its own "Disable compact mode" scenario.
That capability is fully implemented and correctly wired from the setting
through `PanelStyle` to the render decision, but the value the panel reads is
the one its window was born with, so toggling compact tool calls appears to do
nothing until the workspace window is closed and reopened. The feature looks
broken while being entirely present.

Seven other preferences have the same symptom: `agent_panel_shift_enter_sends`,
`markdown_font_size`, `terminal_font_size`, `ui_font_name`, `ui_font_size`,
`mcp_server_enabled` and `restore_conversation_on_launch`.

The write half of #238 - a stale snapshot serialized back over newer values by
a roster write - is out of scope here. Change 341 removed its most frequent
trigger; the remainder is its own change.

## What Changes

- Preferences written by the settings window reach every open workspace
  window, without that window being reopened.
- A workspace window refreshes only the preferences document. Its roster,
  personas, workspace UI state and other collections are left exactly as they
  stand in memory, so a refresh cannot become a second way to lose the roster.
- The refresh happens on a settings write, not on the render path. GPUI
  re-renders per keystroke and `.claude/rules/rust-structure.md` bans I/O
  there.

## Capabilities

### Modified Capabilities

- `settings-persistence`: a preference change applies to already-open windows,
  and reloading preferences preserves the durable collections.

## Impact

- `knot-core`: one new method on `Settings` that re-reads the preferences
  document in place.
- `knot`: the settings window's persist path notifies open windows; the window
  registry can enumerate live workspace views; `WorkspaceWindow` adopts fresh
  preferences.
- No new dependencies. No change to what is stored or to the document layout.

## Non-goals

- The write-side lost update (#238 part 2).
- Sharing one `Settings` value across windows, which would remove the snapshot
  entirely. That is the larger of the two shapes #238 weighs, and it touches
  every window and every persist path; this change fixes the observable defect
  without committing to it.
- Propagating preferences to the workspace manager and command centre windows,
  which hold their own snapshots but draw nothing from the affected scalars.
