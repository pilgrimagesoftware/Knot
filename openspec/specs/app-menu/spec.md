# app-menu Specification

## Purpose

Defines the application menu bar: which menus it carries, what each acts on,
when their items are enabled, and which keys they answer to. Distinct from
the in-app context menus, which act on whatever the pointer is over.

## Requirements

### Requirement: Standard menu items carry the platform's key equivalents

Every item in the menu bar that macOS gives a standard key equivalent SHALL
show that key equivalent, whether or not the behavior behind it is
implemented yet:

| Item | Key |
| --- | --- |
| File > New Workspace | ⌘N |
| File > Close Window | ⌘W |
| Edit > Undo | ⌘Z |
| Edit > Redo | ⇧⌘Z |
| Edit > Cut | ⌘X |
| Edit > Copy | ⌘C |
| Edit > Paste | ⌘V |
| Window > Minimize | ⌘M |
| Help > Knot Help | ⌘? |
| Knot > Settings… | ⌘, |
| Knot > Hide Knot | ⌘H |
| Knot > Hide Others | ⌥⌘H |
| Knot > Quit Knot | ⌘Q |

About Knot, Show All and Zoom SHALL have no key equivalent, because macOS
gives those three none.

Enter Full Screen SHALL NOT appear in this table, and Knot SHALL NOT declare
an item for it or bind ⌃⌘F. The View menu SHALL carry exactly one Enter Full
Screen item, the one macOS adds itself, which is enabled and works.

⌃⌘F is a key the platform reserves, and this specification already forbids a
Knot menu item from holding such a key: a menu item's key equivalent is claimed
by AppKit ahead of the window, so a Knot action on a reserved key takes it
rather than sharing it. The rule that moved Fork Agent off ⌘F applies here
unchanged.

macOS adds its item only when it does not find an equivalent one, and it judges
equivalence by the action behind the item rather than by its label - so a Knot
item it does not recognize is added beside rather than instead, which is how the
menu came to show the same command twice under the same key.

Leaving the item to the platform also leaves it the label that flips between
Enter and Exit Full Screen as the window changes, its translation into the
system's language, and its correct behavior on every window rather than only
the ones Knot opened. This is the one standard item Knot declares nothing for,
because it is the one the platform fully implements.

An item whose behavior is not implemented SHALL still show its key
equivalent, drawn greyed beside the disabled label. That is how macOS
presents a standard item an application does not currently offer, and it
distinguishes "not yet" from "never" - an item with a blank right column
reads as one the application does not have at all.

#### Scenario: A shortcut beside every standard item

- **WHEN** the user opens any menu in the bar
- **THEN** each of its items that macOS gives a standard key equivalent
  shows that key equivalent beside its label

#### Scenario: An unimplemented item still shows its key

- **WHEN** the user opens the Window menu, whose Minimize item has no
  behavior wired to it yet
- **THEN** Minimize is disabled and ⌘M is drawn greyed beside it

#### Scenario: No invented shortcuts

- **WHEN** the user opens the Knot menu or the Window menu
- **THEN** About Knot, Show All and Zoom show no key equivalent

#### Scenario: One Enter Full Screen, and it works

- **WHEN** the user opens the View menu with a workspace window focused
- **THEN** exactly one Enter Full Screen item is listed, it is enabled, and
  choosing it puts that window into full screen

#### Scenario: The item comes back out of full screen

- **WHEN** a window is full screen and the user opens the View menu
- **THEN** the item reads Exit Full Screen, and choosing it returns the window
  to its previous size and position

#### Scenario: The full-screen key still works

- **WHEN** the user presses ⌃⌘F with a workspace window focused
- **THEN** that window enters full screen, and no Knot action intercepts the
  key

#### Scenario: The key reaches every window

- **WHEN** the user presses ⌃⌘F with the settings window focused
- **THEN** the settings window enters full screen

### Requirement: Agents menu

The menu bar SHALL carry an Agents menu offering the same items as the agent
row's context menu, in the same order and with the same grouping, per
`agent-list-ui`. Both menus SHALL be produced from one item set, so an item
added to either appears in both.

The menu SHALL act on the agent currently selected in the focused workspace
window's sidebar, and selecting an item SHALL do exactly what the same item
does from that agent's context menu, including any confirmation it asks for.

#### Scenario: The menu offers what the context menu offers

- **WHEN** the user opens the Agents menu with an agent selected
- **THEN** it lists the same items, in the same order and groups, as that
  agent's context menu

#### Scenario: An item does the same thing from either menu

- **WHEN** the user selects Remove Agent from the Agents menu
- **THEN** the same confirmation appears, and confirming removes the same
  agent, as selecting Remove Agent from that agent's row

### Requirement: Agents menu enablement follows the selection

Every item in the Agents menu SHALL be disabled when no agent is selected in
the focused workspace window, and SHALL become enabled as soon as one is.

An item that does not apply to the selected agent SHALL be shown disabled
rather than omitted. This differs from the context menu, which omits such
items: a context menu is read fresh at the pointer each time, while a menu
bar is navigated from memory, and items that move or vanish between
selections cannot be learned.

#### Scenario: No agent selected

- **WHEN** the user opens the Agents menu with no agent selected - on the
  dashboard, or in a workspace whose selection was cleared
- **THEN** every item is present and disabled

#### Scenario: An item that does not apply to this agent

- **WHEN** the user opens the Agents menu with a shell companion selected
- **THEN** Fork Agent, Duplicate Agent and Register Agent are present and
  disabled, in their usual positions

#### Scenario: Selecting an agent enables the menu

- **WHEN** the user selects an agent in the sidebar
- **THEN** the Agents menu's items that apply to it become enabled without
  the user reopening the window

#### Scenario: No workspace window

- **WHEN** no workspace window is focused - only the workspace manager is
  open
- **THEN** every item in the Agents menu is disabled

### Requirement: Agents menu submenus reflect current state

The Agents menu's submenus - the workspaces an agent can move to, and the
agent's markdown files - SHALL list what is current for the selected agent at
the time the menu is opened, not what was current when the window opened.

#### Scenario: A workspace added since launch

- **WHEN** the user creates a second workspace and then opens the Agents
  menu on an agent in the first
- **THEN** Move to Workspace offers the new workspace

#### Scenario: A markdown file shown since launch

- **WHEN** an agent displays a markdown file and the user then opens the
  Agents menu on that agent
- **THEN** Markdown Files lists it

### Requirement: The Agents menu's shortcuts come from the reference

No platform convention names a key for the Agents menu's items - they are
Knot's own - so the Swift reference is the source. The menu SHALL carry the
shortcuts it gives them:

| Item | Key |
| --- | --- |
| New Shell Companion | ⇧⌘S |
| Fork Agent | ⌥⌘F |
| Duplicate Agent | ⌘D |
| Restart Agent | ⌘R |

Where the reference wants a key the platform has already spoken for, the
platform SHALL win, and no item of this menu SHALL hold a key the platform
reserves - including one the port has no item for yet. A menu item's key
equivalent is claimed by AppKit ahead of the window, so a Knot action on
such a key does not merely share it, it takes it.

Fork Agent SHALL therefore take ⌥⌘F rather than the reference's ⌘F, and ⌘F
SHALL stay free for a find.

Remove Agent SHALL have no key equivalent at all. The reference calls that
item Close Agent and gives it ⌘W, which belongs to Close Window here; the
losing item goes without rather than taking a second-choice key - the more
so as Remove Agent is the destructive one.

Every other item in the menu SHALL have none: the reference gives them
none either.

A shortcut SHALL do exactly what the menu item does, which includes doing
nothing when the item is disabled - no agent selected, or one the item does
not apply to.

#### Scenario: Restarting the selected agent by keyboard

- **WHEN** an agent is selected in the focused workspace window and the
  user presses ⌘R
- **THEN** the same confirmation appears, and confirming restarts the same
  agent, as choosing Agents > Restart Agent

#### Scenario: A shortcut for an item that does not apply

- **WHEN** a shell companion is selected - Fork Agent is shown disabled for
  it - and the user presses ⌥⌘F
- **THEN** nothing happens, matching the disabled item

#### Scenario: The find key is left alone

- **WHEN** the user presses ⌘F anywhere in the application
- **THEN** no Agents menu item runs, and the key remains available to a
  find

#### Scenario: A shortcut with no agent selected

- **WHEN** no agent is selected, or no workspace window is focused, and the
  user presses ⌘D
- **THEN** nothing happens

#### Scenario: Remove Agent has no shortcut

- **WHEN** the user opens the Agents menu
- **THEN** Remove Agent shows no key equivalent, and ⌘W remains Close
  Window

### Requirement: The Edit menu operates the focused text field

The Edit menu's Undo, Redo, Cut, Copy and Paste items SHALL dispatch the
text actions of the focused text field, and SHALL be enabled exactly when a
text field is focused.

They SHALL NOT take their keys away from anything else. Where no text field
claims them - the terminal pane, which answers ⌘C with its own copy-selection
- the items are disabled, and the key reaches that handler as it did before.

#### Scenario: Copying from a text field

- **WHEN** the user selects text in a field in the settings window and
  chooses Edit > Copy
- **THEN** the selection is copied, exactly as ⌘C in that field does

#### Scenario: The Edit menu with no text field focused

- **WHEN** the user focuses a terminal pane and opens the Edit menu
- **THEN** all five items are disabled

#### Scenario: The terminal keeps its copy key

- **WHEN** the user selects terminal output and presses ⌘C
- **THEN** the selection is copied by the terminal pane, unaffected by the
  Edit menu's claim on that key

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

### Requirement: The Window menu's own items precede the window list

The Window menu SHALL list Knot's own items - Command Center, Workspaces,
Minimize and Zoom - before the list of open windows macOS appends to that
menu, with a separator between Knot's items and that list.

Command Center and Workspaces SHALL come first, as a group separated from
Minimize and Zoom: the first pair opens windows, the second pair manipulates
the focused one, and they are not each other's neighbours.

macOS appends and maintains the trailing list of open windows itself, so Knot's
items SHALL be declared in that order rather than positioned after the fact.
Without the separator, a workspace called "Zoom" is indistinguishable from the
Zoom command, and the menu's fixed items shift down every time a window opens.

#### Scenario: Order in the menu

- **WHEN** the user opens the Window menu
- **THEN** Command Center and Workspaces appear first, then a separator, then
  Minimize and Zoom, then a separator, then the open windows

#### Scenario: The fixed items do not move

- **WHEN** the user opens three more workspace windows and opens the Window
  menu again
- **THEN** Command Center, Workspaces, Minimize and Zoom are in the same
  positions, and the three new windows are listed below the last separator
