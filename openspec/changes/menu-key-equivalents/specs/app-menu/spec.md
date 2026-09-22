# Spec Delta

## ADDED Requirements

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
| View > Enter Full Screen | ⌃⌘F |
| Window > Minimize | ⌘M |
| Help > Knot Help | ⌘? |
| Knot > Settings… | ⌘, |
| Knot > Hide Knot | ⌘H |
| Knot > Hide Others | ⌥⌘H |
| Knot > Quit Knot | ⌘Q |

About Knot, Show All and Zoom SHALL have no key equivalent, because macOS
gives those three none. The Agents menu's items SHALL have none either:
they are Knot's own, so no platform convention names a key for them.

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

- **WHEN** the user opens the Knot menu, or the Agents menu
- **THEN** About Knot, Show All and every Agents item show no key
  equivalent

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
