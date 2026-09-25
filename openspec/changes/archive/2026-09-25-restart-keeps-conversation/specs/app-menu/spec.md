# Spec Delta

## MODIFIED Requirements

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
| Restart with New Conversation | ⇧⌘R |

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

Restart with New Conversation has no counterpart in the reference, which
always starts a new conversation on restart. It SHALL take ⇧⌘R, Restart
Agent's key with Shift added, since it is that item's other half.

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

#### Scenario: Starting over by keyboard

- **WHEN** a non-shell agent is selected in the focused workspace window and
  the user presses ⇧⌘R
- **THEN** the same confirmation appears, and confirming restarts the agent in
  a new conversation, as choosing Agents > Restart with New Conversation
