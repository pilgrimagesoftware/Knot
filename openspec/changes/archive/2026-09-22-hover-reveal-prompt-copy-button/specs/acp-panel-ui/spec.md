# Spec Delta

## ADDED Requirements

### Requirement: Prompt cell copy control

Each user message in the panel SHALL expose a copy control that places the
message's text on the system clipboard. The control SHALL be revealed on
hover of the message and SHALL copy exactly the prompt text, without the
bubble's surrounding chrome and without any attached-context payload.

The hover target SHALL be the prompt and its control together, as one region.
The control sits beside the bubble rather than inside it, so a target limited to
the bubble would hide the control at the moment the pointer reached it; moving
the pointer from the bubble onto the control SHALL keep it shown, and it SHALL
stay shown for as long as the pointer rests on it.

The hover target SHALL NOT extend beyond that region. A user message is
right-aligned within a full-width row, and pointing at the empty space beside it
SHALL reveal nothing.

Revealing and hiding the control SHALL NOT move anything on screen. Its space
SHALL be reserved whether it is shown or not, so a pointer travelling down a
conversation does not make the messages shift as it passes them.

The control SHALL NOT be drawn for anything but a user message, and hovering one
user message SHALL reveal only that message's control.

#### Scenario: Copying a prompt
- **WHEN** the user activates the copy control on a user message while
  hovering it
- **THEN** the message's text is placed on the system clipboard

#### Scenario: Attachments are not part of the copy
- **WHEN** the user copies a prompt that had files or images attached when it
  was sent
- **THEN** the clipboard receives the prompt text alone, with none of the
  attachment payload

#### Scenario: The control is not persistent chrome
- **WHEN** the pointer leaves the user message
- **THEN** the copy control is no longer shown, and the bubble itself is
  unchanged

#### Scenario: The control survives being pointed at
- **WHEN** the pointer moves from a user message's bubble onto the revealed copy
  control
- **THEN** the control stays shown and can be activated

#### Scenario: Empty space beside a prompt reveals nothing
- **WHEN** the pointer rests in the empty area to the left of a right-aligned
  user message, on the same line
- **THEN** no copy control is shown

#### Scenario: Hovering does not move the conversation
- **WHEN** the pointer moves down a conversation across several user messages
- **THEN** each control appears and disappears in place, and no message, bubble
  or surrounding content shifts position

#### Scenario: Only the hovered message reveals its control
- **WHEN** a conversation holds several user messages and the pointer rests on
  one of them
- **THEN** only that message's copy control is shown
