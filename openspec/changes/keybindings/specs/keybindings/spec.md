# Spec Delta

## Purpose

Defines the keyboard shortcuts that navigate Knot - between workspace windows,
between agents, to an agent's input and to a workspace's panels - and how a
user customizes them.

## ADDED Requirements

### Requirement: Navigation shortcuts and their defaults

The application SHALL provide these shortcuts, bound to these defaults until
the user changes them:

| Shortcut | Default |
| --- | --- |
| Select workspace 1–9 | ⌘1 … ⌘9 |
| Select agent 1–9 | ⌥⌘1 … ⌥⌘9 |
| Focus agent input | ⌘L |
| Toggle Dashboard | ⌥⌘O |
| Toggle Pull Requests | ⌥⌘P |
| Jump to bottom | ⌃⌘↓ |
| Open Command Center | ⌥⌘0 |

The two numbered families SHALL always use the digits 1 through 9; only their
modifier is configurable. Each of the other five is a single configurable
chord.

The Swift reference binds ⌘1–⌘9 to workspace switching and ⌘0 to the Command
Center. The port keeps ⌘1–⌘9 and the existing ⌥⌘0, because ⌘0 already opens
the workspace manager here. Numbered agent selection, Focus agent input, Jump
to bottom and the two panel toggles have no reference counterpart.

Jump to bottom defaults to ⌃⌘↓ rather than the platform's ⌘↓. The composer,
which usually has focus, uses ⌘↓, ⇧⌘↓ and ⌥⌘↓ for its own caret movement, and
takes them first while it is focused.

#### Scenario: Defaults on first launch

- **WHEN** the user has never customized a shortcut
- **THEN** ⌘3 selects workspace 3, ⌥⌘2 selects agent 2, ⌘L focuses the
  selected agent's input, ⌥⌘O toggles the Dashboard, ⌥⌘P toggles Pull
  Requests, ⌃⌘↓ jumps to the bottom of the conversation, and ⌥⌘0 opens the
  Command Center

### Requirement: Selecting a workspace by number

Pressing the workspace modifier with digit N SHALL open the Nth workspace, in
the order the workspace manager lists workspaces. If that workspace's window is
already open, the window SHALL be raised and focused instead, so that one
window per workspace is kept (`window-lifecycle`). The shortcut SHALL work from
any Knot window, and with no Knot window focused. When there are fewer than N
workspaces, it SHALL do nothing.

#### Scenario: Raising an open workspace

- **WHEN** workspaces A, B and C are listed in that order, B's window is open
  behind A's, and the user presses ⌘2
- **THEN** B's window comes to the front and is focused, and no second window
  for B opens

#### Scenario: Opening a closed workspace

- **WHEN** workspace C has no open window and the user presses ⌘3
- **THEN** C's window opens

#### Scenario: Out of range

- **WHEN** there are two workspaces and the user presses ⌘5
- **THEN** nothing happens

### Requirement: Selecting an agent by number

Pressing the agent modifier with digit N in a workspace window SHALL select the
Nth agent in that window's sidebar, top to bottom, and show it. Showing it
leaves a Dashboard or Pull Requests panel and returns to the agent view, the
same as clicking the agent's row. The shortcut SHALL act only on the focused
workspace window. It SHALL do nothing in any other window, and when the
workspace has fewer than N agents.

#### Scenario: Selecting the second agent

- **WHEN** a workspace window lists agents X, Y and Z, and the user presses ⌥⌘2
- **THEN** Y is selected and its terminal or panel is shown

#### Scenario: Leaving a panel

- **WHEN** the Dashboard is showing and the user presses ⌥⌘1
- **THEN** the first agent is selected and shown in place of the Dashboard

#### Scenario: Outside a workspace window

- **WHEN** the Command Center is focused and the user presses ⌥⌘1
- **THEN** nothing happens

### Requirement: Focusing the agent's input

The Focus agent input shortcut SHALL move keyboard focus to the selected
agent's input in the focused workspace window. That is the composer for an
agent in panel mode and the terminal for an agent in terminal mode. The
shortcut SHALL work even when focus is already elsewhere in the window, such
as in a search field or the git panel. If a Dashboard or Pull Requests panel is
showing, the window SHALL first return to the agent view. With no agent
selected, or outside a workspace window, the shortcut SHALL do nothing.

#### Scenario: Returning to the composer

- **WHEN** a panel-mode agent is selected, focus is in the git panel's commit
  field, and the user presses ⌘L
- **THEN** the agent's composer has keyboard focus

#### Scenario: From the Dashboard

- **WHEN** the Dashboard is showing and the user presses ⌘L
- **THEN** the selected agent's view is shown and its input has focus

### Requirement: Jumping to the bottom of the conversation

The Jump to bottom shortcut SHALL scroll the selected agent's conversation in
the focused workspace window to its latest output, and SHALL resume following
new output, the same as the conversation's "Scroll to latest" control. It
applies to an agent in panel mode. A terminal-mode agent's pane always shows
the bottom of its output, so there the shortcut SHALL do nothing. It SHALL
also do nothing:

- while a Dashboard or Pull Requests panel is showing;
- with no agent selected;
- outside a workspace window.

#### Scenario: Back to the latest output

- **WHEN** the user has scrolled a panel-mode agent's conversation up and
  presses ⌃⌘↓
- **THEN** the conversation shows its latest output and follows new output
  as it arrives

#### Scenario: With a panel showing

- **WHEN** the Dashboard is showing and the user presses ⌃⌘↓
- **THEN** the Dashboard stays, and the conversation behind it does not move

### Requirement: Toggling the workspace panels

The Toggle Dashboard and Toggle Pull Requests shortcuts SHALL behave like the
workspace sidebar's Dashboard and Pull Requests rows in the focused workspace
window. Each shows its panel, and hides it if it is already showing, which
returns the window to the agent view. Outside a workspace window they SHALL do
nothing.

#### Scenario: Toggling the Pull Requests panel

- **WHEN** the agent view is showing and the user presses ⌥⌘P twice
- **THEN** the first press shows the Pull Requests panel and the second
  returns to the agent view

#### Scenario: Switching panels

- **WHEN** the Dashboard is showing and the user presses ⌥⌘P
- **THEN** the Pull Requests panel replaces the Dashboard

### Requirement: Opening the Command Center by shortcut

The Open Command Center shortcut SHALL open the Command Center, or raise it if
it is already open. It SHALL be the key equivalent shown beside Window >
Command Center (`app-menu`).

#### Scenario: Customized key reaches the menu

- **WHEN** the user rebinds Open Command Center to ⌃⌘K
- **THEN** Window > Command Center shows ⌃⌘K, ⌃⌘K opens the Command Center,
  and ⌥⌘0 no longer does

### Requirement: Customizing the shortcuts

The settings window's Keyboard tab SHALL list every shortcut in this
capability, each with its current binding and a control to reset it to its
default, and a control that resets all of them. It SHALL provide:

- For each numbered family, a choice of modifier made from any combination of
  ⌃, ⌥, ⇧ and ⌘.
- For each single-chord shortcut, a recorder. Once armed, the recorder takes
  the next chord pressed as the new binding. Escape cancels it and leaves the
  binding unchanged.

An accepted change SHALL be persisted immediately as a preference and SHALL
take effect in every open window without a restart. The previous chord SHALL
stop triggering the shortcut.

#### Scenario: Changing the agent modifier

- **WHEN** the user changes the agent-selection modifier from ⌥⌘ to ⌃⌘
- **THEN** ⌃⌘2 selects the second agent, ⌥⌘2 no longer does, and the setting
  survives a relaunch

#### Scenario: Recording a chord

- **WHEN** the user arms the Focus agent input recorder and presses ⌃⌘I
- **THEN** Focus agent input is bound to ⌃⌘I and ⌘L no longer triggers it

#### Scenario: Cancelling a recording

- **WHEN** the user arms a recorder and presses Escape
- **THEN** the binding is unchanged

#### Scenario: Resetting

- **WHEN** the user resets Toggle Dashboard after rebinding it
- **THEN** it is bound to ⌥⌘O again

### Requirement: A customization cannot break other shortcuts

The Keyboard tab SHALL reject, and not persist, a binding in any of these
cases:

- A single chord that has none of ⌘, ⌃ and ⌥. Such a chord would take a key
  away from typing.
- A numbered-family modifier that has none of ⌘, ⌃ and ⌥.
- A binding, or any of a family's nine chords, that equals a chord already
  used by another Knot shortcut. This covers the other shortcuts in this
  capability and Knot's fixed shortcuts, such as ⌘Q, ⌘W or the Agents menu's.

A rejected binding SHALL leave the previous binding in effect, and the tab
SHALL say why it was rejected, naming the conflicting shortcut when there is
one.

A stored binding that fails these rules, or that does not parse, SHALL be
ignored at launch in favour of its default. A hand-edited or older preference
document must not leave Knot without its shortcuts.

#### Scenario: Conflict with a fixed shortcut

- **WHEN** the user records ⌘W for Toggle Dashboard
- **THEN** the binding is rejected with a message naming Close Window, and ⌥⌘O
  still toggles the Dashboard

#### Scenario: Families collide

- **WHEN** the workspace modifier is ⌘ and the user sets the agent modifier to ⌘
- **THEN** the change is rejected with a message naming Select workspace

#### Scenario: Unmodified chord

- **WHEN** the user records the single key L for Focus agent input
- **THEN** the binding is rejected, and the message says a ⌘, ⌃ or ⌥ modifier
  is required

#### Scenario: Invalid stored binding

- **WHEN** the preferences document stores `"cmd-q"` for Toggle Dashboard and
  Knot launches
- **THEN** Toggle Dashboard is bound to ⌥⌘O and ⌘Q still quits
