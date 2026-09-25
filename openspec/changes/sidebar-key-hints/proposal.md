# Proposal

## Why

The sidebar's controls all have keyboard shortcuts now, but nothing on screen
says which. The View menu (`view-menu-navigation`) lists them, but the user
has to open it to find out. The New agent control has no shortcut at all.

## What Changes

- Holding ⌘ in a workspace window for 500 ms shows each sidebar control's key
  beside it: the Dashboard and Pull Requests rows, the first nine agent rows,
  and the New agent control. The hints follow rebindings, do not flash on an
  ordinary ⌘ shortcut, and never resize a row.
- New fixed shortcut **⌘T** for New Agent, from the Swift reference, with a
  File > New Agent… item enabled only in a workspace window.

The hints diverge from the Swift reference, whose sidebar shows no shortcuts.

Non-goals:
- Hints outside the sidebar.
- Making the New Agent key configurable.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-list-ui`: holding ⌘ shows the sidebar controls' shortcuts.
- `app-menu`: File > New Agent… (⌘T).

## Impact

- `crates/knot/src/keymap/fixed.rs`: the ⌘T binding, which the validator then
  rejects as a customization target.
- `crates/knot/src/app_bootstrap.rs`: the File menu item.
- `crates/knot/src/workspace_window`: the New Agent handler, ⌘-hold tracking,
  and hint rendering (`render/sidebar.rs`, `render/mod.rs`).
