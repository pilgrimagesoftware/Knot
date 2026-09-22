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
gives those three none.

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

### Requirement: The Agents menu's shortcuts come from the reference

No platform convention names a key for the Agents menu's items - they are
Knot's own - so the Swift reference is the source. The menu SHALL carry the
shortcuts it gives them:

| Item | Key |
| --- | --- |
| New Shell Companion | ⇧⌘S |
| Fork Agent | ⌘F |
| Duplicate Agent | ⌘D |
| Restart Agent | ⌘R |

Remove Agent SHALL have no key equivalent. The reference calls that item
Close Agent and gives it ⌘W, which belongs to Close Window here; where the
two sources disagree over a key this common, the platform wins, and the
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
  it - and the user presses ⌘F
- **THEN** nothing happens, matching the disabled item

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
