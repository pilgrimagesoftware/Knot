# Proposal

## Why

The workspace name dialog in the Workspaces window is mouse-only. Pressing
Return after typing a name does nothing, and Escape does not dismiss it - the
user has to reach for the pointer to finish an interaction they started on the
keyboard. The Swift reference binds both keys
(`Skwad/Views/Workspace/WorkspaceSheet.swift:86-100`), so this is a port gap
rather than a new idea, and it is the kind of gap that makes a dialog feel
broken rather than merely unfinished.

## What Changes

- Return in the workspace name dialog SHALL do what the Create (or Save)
  button does.
- Escape SHALL do what Cancel does: close the dialog and discard the typed
  name.
- Return with an empty or whitespace-only name SHALL do nothing, matching the
  reference, where the button carrying the Return shortcut is disabled on that
  condition. The dialog stays open; no error is raised by the key itself.
- The Create/Save button SHALL be visibly disabled while the name is empty or
  whitespace-only, so the key's inertness is explained rather than read as the
  dialog being stuck.

The dialog serves both modes - New Workspace and Rename Workspace - from one
code path, so both gain the keys together.

## Capabilities

### New Capabilities

- `workspace-manager-ui`: the Workspaces window - its workspace list, the
  name dialog it opens for creating and renaming, and how that dialog is
  operated. Nothing specifies this window today; `settings-ui` covers the
  settings window only, and `agent-list-ui` covers the workspace *window's*
  sidebar, not the workspace manager. This change gives the capability a
  contract seeded with the dialog's keyboard behavior; the rest of the window
  can be written down as it is touched.

### Modified Capabilities

(none)

## Impact

- `crates/knot`: `workspace_manager/mod.rs` - a key handler on the dialog
  overlay routed to the existing `confirm_workspace_dialog` and
  `cancel_workspace_dialog`, and a disabled state on the confirm button
  derived from the name input.
- No change to `knot-core` or any crate below the UI. Both actions already
  exist and are already reachable by click; this adds a second way to reach
  them.

## Non-Goals

- The Delete Workspace confirmation overlay in the same window. It has the
  same gap, but binding Return to a destructive default is a separate decision
  that should not ride along with this one.
- Keyboard operation of the workspace list itself (arrow-key navigation,
  Return to open, Delete to remove). Worth having, out of scope here.
- Any change to what the dialog contains. The reference also offers a color
  swatch grid the port does not have; that is a separate gap.
