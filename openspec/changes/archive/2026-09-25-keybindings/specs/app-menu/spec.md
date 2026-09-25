# Spec Delta

## MODIFIED Requirements

### Requirement: The Window menu opens Knot's two global windows

The Window menu SHALL carry an item that opens the Command Center and an item
that opens the workspace manager, each with a key equivalent:

| Item | Key |
| --- | --- |
| Window > Command Center | the Open Command Center shortcut (default ⌥⌘0) |
| Window > Workspaces | ⌘0 |

The Command Center's key equivalent is user-configurable (`keybindings`). The
item SHALL show the current binding, and SHALL update when the user changes
it, without a restart. Workspaces keeps the fixed ⌘0.

Both SHALL be enabled at all times, including when no window is open at all.
They are how a user gets back to a window, so an enablement rule that depends
on a window being focused would disable them exactly when they are needed.

Choosing either SHALL open that window, or activate it if it is already open,
per `window-lifecycle`. The workspace manager is likewise a single window.
Their key equivalents SHALL do exactly what the items do.

The Command Center had no menu item and no shortcut, reachable only from a
toolbar button in the workspace manager - which is unreachable itself once the
manager is behind something else.

#### Scenario: Opening the Command Center from the menu

- **WHEN** the user chooses Window > Command Center
- **THEN** the Command Center window opens, or comes to the front if it was
  already open

#### Scenario: Opening the Command Center by keyboard

- **WHEN** the user presses ⌥⌘0 with the default binding in effect
- **THEN** the same thing happens as choosing Window > Command Center

#### Scenario: The menu follows a rebinding

- **WHEN** the user rebinds Open Command Center to ⌃⌘K and opens the Window menu
- **THEN** Window > Command Center shows ⌃⌘K

#### Scenario: Getting back to the manager

- **WHEN** the workspace manager is open behind two workspace windows and the
  user presses ⌘0
- **THEN** the manager comes to the front and is focused

#### Scenario: Reachable with nothing focused

- **WHEN** every Knot window is closed and the user opens the Window menu
- **THEN** Command Center and Workspaces are both enabled
