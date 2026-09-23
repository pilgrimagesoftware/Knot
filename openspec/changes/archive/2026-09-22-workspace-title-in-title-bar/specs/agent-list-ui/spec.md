# Spec Delta

## ADDED Requirements

### Requirement: The sidebar's title bar names the workspace

The sidebar's title bar — the strip that holds the workspace window's traffic
lights — SHALL show the application icon followed by the name of the workspace
the window belongs to. It SHALL NOT show the application's name: a workspace
window is one of several the user may have open, and the one thing that tells
them apart is the workspace.

This diverges from the Swift reference, whose workspace window shows the
application name here. It aligns the window with its siblings, which already
name themselves in the same position — the Command Center and the Workspaces
window — and with this window's own OS-level title, which already carries the
workspace name and is what Mission Control, Cmd+` and the Window menu show.

The name SHALL be the workspace's current name, not the name it had when the
window opened: renaming a workspace SHALL update the title bar of that
workspace's open window, and that window's OS title, without the window being
closed and reopened, and SHALL leave every other open window's title unchanged.

The name SHALL occupy one line. A name too long for the available width SHALL
be truncated with an ellipsis rather than wrapping, pushing the title bar
taller, or displacing the traffic lights.

#### Scenario: The window says which workspace it is

- **WHEN** a workspace window is open for a workspace named "Payments"
- **THEN** its title bar shows the application icon followed by "Payments", and
  does not show the application's name

#### Scenario: Two open windows are distinguishable

- **WHEN** windows are open for two different workspaces
- **THEN** each title bar shows its own workspace's name

#### Scenario: A rename reaches the open window

- **WHEN** the user renames a workspace in the Workspaces window while that
  workspace's window is open
- **THEN** the open window's title bar shows the new name, and so does its OS
  window title, with no reopen

#### Scenario: A rename leaves other windows alone

- **WHEN** the user renames one workspace while windows for two workspaces are
  open
- **THEN** only the renamed workspace's window changes its title

#### Scenario: A long name is truncated

- **WHEN** the workspace's name is wider than the space the title bar has for it
- **THEN** the name is truncated with an ellipsis on one line, and the traffic
  lights stay where they are

## MODIFIED Requirements

### Requirement: A narrow sidebar shows a compact layout

Below a compact breakpoint of 160px the sidebar SHALL show a compact layout
rather than squeezing the full-width one, porting the Swift reference's
`isCompact` sidebar:

- An agent row SHALL show its avatar alone, centered, with the state dot
  overlaid on the avatar rather than in a column of its own. The name, agent
  type, companion marker, persona, activity line and folder SHALL be hidden, and
  the agent's name SHALL be available as the row's tooltip so a row stays
  identifiable.
- The dashboard row SHALL show its icon alone, centered, with its label hidden.
- The sidebar's title bar SHALL hide the workspace name and keep the application
  icon. The workspace SHALL stay identifiable while the label is hidden: the
  window's OS title still carries the name, and the title bar SHALL offer the
  name as a tooltip, the same way a compact agent row does.
- The new-agent control SHALL show its icon alone, with its label hidden and its
  meaning available as a tooltip.

Everything that is not text SHALL behave as it does at full width: selection
highlighting, the dimming of an agent that is not running, the companion
indent, and every context menu and click target named elsewhere in this
capability.

At or above the breakpoint the sidebar SHALL show the full-width layout
unchanged.

#### Scenario: Crossing the breakpoint

- **WHEN** the user drags the sidebar from 250px down to 140px
- **THEN** the rows become avatar-only, the dashboard row icon-only, and the
  new-agent control icon-only

#### Scenario: Crossing back

- **WHEN** the user drags a compact sidebar back to 200px
- **THEN** every row shows its name and detail lines again

#### Scenario: The compact title bar is still identifiable

- **WHEN** the sidebar is compact and the pointer rests on its title bar
- **THEN** a tooltip names the workspace

#### Scenario: A compact row is still identifiable

- **WHEN** the pointer rests on a compact agent row
- **THEN** a tooltip names that agent

#### Scenario: A compact row still reports state

- **WHEN** an agent is working and the sidebar is compact
- **THEN** its state dot is visible on its avatar

#### Scenario: A stopped agent still reads as stopped when compact

- **WHEN** a workspace holds a running and a stopped agent and the sidebar is
  compact
- **THEN** the stopped agent's row is still visually distinguishable from the
  running one's

#### Scenario: Compact rows keep their menus

- **WHEN** the user right-clicks a compact agent row
- **THEN** the agent row context menu opens with the same items it has at full
  width

#### Scenario: Selection still shows

- **WHEN** an agent is selected and the sidebar is compact
- **THEN** its row is highlighted as the selected row
