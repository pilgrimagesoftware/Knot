# Spec Delta

## ADDED Requirements

### Requirement: Prompt cell copy control
Each user message in the panel SHALL expose a copy control that places the
message's text on the system clipboard. The control SHALL be revealed on
hover of the message and SHALL copy exactly the prompt text, without the
bubble's surrounding chrome and without any attached-context payload.

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