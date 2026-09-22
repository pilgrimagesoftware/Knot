# Proposal

## Why

A workspace window's title bar says "Knot" beside the traffic lights — the one
thing the user already knows, and the one thing that cannot tell two open
windows apart. Every sibling window names itself there (the Command Center says
"Command Center", the Workspaces window says "Workspaces"); the workspace
window, the one there can be several of at once, is the only one that does not.
The name it should show is already resolved and already used: `open.rs` sets the
same string as the OS window title, which is what Mission Control, Cmd+` and the
Window menu show, so the window's two identities currently disagree with each
other.

## What Changes

- The workspace window's sidebar title bar shows the workspace's name beside the
  application icon, in place of the application name.
- The title tracks the workspace: renaming a workspace in the Workspaces window
  updates the title bar of that workspace's open window, and its OS window
  title, without reopening it.
- Compact layout is unchanged in effect: below the 160px breakpoint the label is
  hidden and the icon kept, exactly as it is today — only the label's content
  differs.

Non-goals:

- The Command Center and Workspaces windows keep naming themselves; nothing
  about their title bars changes.
- No new title-bar content beyond the name: no workspace color swatch, no agent
  count, no truncation indicator beyond the ellipsis the row already applies.
- The content column's own header — the selected agent's avatar, name, folder
  and state — is a separate surface and is not touched.
- The deleted-workspace path (the window that renders `workspace.missing` with
  an empty title bar) is unchanged.

## Capabilities

### New Capabilities

None. The sidebar title bar is already part of an existing capability.

### Modified Capabilities

- `agent-list-ui`: the sidebar's title bar gains a requirement saying what it
  shows at full width — the application icon and the workspace's name — where
  today only the compact-layout requirement mentions its contents, and does so
  by referring to an "application-name label" that this change removes. That
  bullet is reworded, and the new requirement carries the rename-tracking
  behavior.

## Impact

- `crates/knot/src/workspace_window/render/mod.rs` — `sidebar_title_bar` is an
  associated function taking only `compact` and `cx`; it needs the window's
  workspace name, so it becomes a method reading the shared store by
  `workspace_id` rather than taking a snapshot (see issue #238 for what a
  snapshot field costs here).
- `crates/knot/src/workspace_manager/mod.rs` — the rename path persists and
  notifies its own window only. Reaching another window's paint needs
  `cx.refresh_windows()`, and the renamed workspace's OS window title needs
  setting; both are new calls on that path.
- `crates/knot-core/locales/en.yml` — `app.name` loses its only use in the
  workspace window. It stays: the About window and the OS-level application name
  still use it.
- No new dependency, no persistence change, no new l10n key — the workspace name
  is user data, not localized text.
