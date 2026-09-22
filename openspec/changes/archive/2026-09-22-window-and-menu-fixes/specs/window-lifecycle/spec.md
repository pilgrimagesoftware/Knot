# Spec Delta

## Purpose

Governs how many windows a given target may have open at once, how a repeat
request to open one resolves to raising the window that already exists, and how
a window whose position was remembered is reconciled with the displays actually
attached before it is shown.

## ADDED Requirements

### Requirement: A workspace has at most one window

A request to open a workspace SHALL open a window for it only when that
workspace has none. When a window for that workspace is already open, the
request SHALL activate that window - bringing it in front of the application's
other windows and making it the focused one - and SHALL NOT open a second.

This SHALL hold for every route that opens a workspace: the workspace manager's
list, whether by double-click or by its open control; an agent card in the
Command Center; a workspace heading in the Command Center; and the jump that
follows creating an agent. A user cannot tell which route they took from the
window they get, so the routes SHALL NOT differ in how many they produce.

When the request names an agent as well as a workspace - a card click does,
a workspace heading does not - activating an existing window SHALL also select
that agent in it, leaving the window showing what a freshly opened one would
have shown. An activated window SHALL otherwise keep its current state: its
sidebar width, its scroll positions, its running sessions and its view mode are
not reset by being raised.

Closing a workspace's window SHALL return that workspace to having none, so the
next request opens a window again rather than failing to find one to activate.

This diverges from the Swift reference, which opens a window per request.

#### Scenario: Opening a workspace that is already open

- **WHEN** a workspace's window is open, possibly behind other windows, and
  the user double-clicks that workspace in the manager
- **THEN** the existing window is brought to the front and focused, and no
  second window for that workspace appears

#### Scenario: Opening from a different route

- **WHEN** a workspace's window was opened from the manager, and the user then
  clicks a card for one of that workspace's agents in the Command Center
- **THEN** the same window is brought to the front, with that agent selected,
  and no second window appears

#### Scenario: Repeating the request many times

- **WHEN** the user opens the same workspace ten times in a row
- **THEN** exactly one window for that workspace exists

#### Scenario: An activated window keeps its state

- **WHEN** the user has widened the sidebar and scrolled an agent's
  conversation, then activates that window again from the manager
- **THEN** the sidebar keeps its width and the conversation its scroll position

#### Scenario: A closed window can be opened again

- **WHEN** the user closes a workspace's window and then opens that workspace
  from the manager
- **THEN** a window opens

#### Scenario: Different workspaces still get different windows

- **WHEN** two workspaces are each opened
- **THEN** two windows are open, one per workspace

### Requirement: The Command Center is a single window

A request to open the Command Center SHALL activate the existing Command Center
window when one is open, and SHALL open one only when none is. Closing it
SHALL return it to having none.

The Command Center shows every workspace at once, so a second copy shows
exactly what the first does; two of them are never what the user meant.

#### Scenario: Opening the Command Center twice

- **WHEN** the Command Center is open and the user opens it again
- **THEN** the existing window is brought to the front and focused, and no
  second Command Center window appears

#### Scenario: Reopening after closing

- **WHEN** the user closes the Command Center and opens it again
- **THEN** a Command Center window opens

### Requirement: A restored window opens where the user can reach it

A window whose position and size were remembered SHALL be placed within the
displays attached at the time it opens. Before the remembered bounds are used,
they SHALL be reconciled against those displays so that the window's title bar
and a graspable part of its top edge are visible on one of them.

Reconciliation SHALL preserve as much of the remembered geometry as it can:

- Remembered bounds that already lie within an attached display SHALL be used
  unchanged.
- Bounds that lie partly off every attached display SHALL be moved - not
  resized - by the smallest offset that brings the required part on screen,
  onto the display they overlap most.
- Bounds that lie wholly off every attached display, or whose size exceeds the
  largest attached display, SHALL fall back to the same default placement a
  workspace window with no remembered bounds gets.

A window SHALL NOT be resized to fit unless its remembered size does not fit
any attached display, because a size the user chose is information and a
position they can no longer reach is not.

Reconciling a window's bounds on open SHALL NOT overwrite what is remembered.
The remembered bounds are rewritten only when the user moves or resizes the
window, so reconnecting the display the window was arranged on restores the
arrangement rather than the fallback.

The Swift reference restores remembered bounds verbatim, which is how a window
reached a position the user could not.

#### Scenario: The display is still attached

- **WHEN** a workspace window was last positioned on a display that is still
  attached and the user opens that workspace
- **THEN** the window opens at exactly its remembered position and size

#### Scenario: The display is gone

- **WHEN** a workspace window's remembered bounds lie on a display that is no
  longer attached, and the user opens that workspace
- **THEN** the window opens fully within an attached display, with its title
  bar visible and grabbable

#### Scenario: Partly off the edge

- **WHEN** a window's remembered bounds hang off the right edge of the only
  attached display
- **THEN** the window opens at its remembered size, moved just far enough left
  for its title bar to be visible

#### Scenario: Larger than any display

- **WHEN** a window's remembered size is larger than every attached display
- **THEN** the window opens at the default size and position

#### Scenario: Reconciliation is not written back

- **WHEN** a window is opened with reconciled bounds and closed without the
  user moving or resizing it, and the missing display is then reattached
- **THEN** opening that workspace again restores the original remembered
  bounds on that display
