# Spec Delta

## ADDED Requirements

### Requirement: Input area context indicator
The input area SHALL display in its bottom bar an always-visible indicator
summarizing the files and images attached to the pending message, counting
files and images separately, whether or not any items are attached. The
indicator's position SHALL be fixed regardless of how many items are attached.
The indicator SHALL identify each attached item on demand (for example in a
tooltip), and in the zero state SHALL read as "no context".

#### Scenario: Attaching items updates the indicator
- **WHEN** the user attaches two files and one image to the pending message
- **THEN** the bottom bar's context indicator shows a file count of two and
  an image count of one while those items remain attached

#### Scenario: The indicator stays put when empty
- **WHEN** the pending message has no attached items
- **THEN** the context indicator still occupies its place in the bottom bar
  and reads as the zero state, and the other bottom-bar controls do not move

#### Scenario: The attached names are reachable
- **WHEN** the user focuses the context indicator while items are attached
- **THEN** the names of the attached files and images are shown

### Requirement: Input area clear-all context control
The input area SHALL provide a clear-all control on the context indicator
that detaches every files and images attached to the pending message at once.
The control SHALL be inoperative while nothing is attached.

#### Scenario: Clearing all context at once
- **WHEN** the user activates the clear-all control with several items
  attached to the pending message
- **THEN** every attached item is detached, the chips row disappears, the
  context indicator shows the zero state, and the cleared attachments are not
  sent with the next message

#### Scenario: Clear-all with nothing attached
- **WHEN** the pending message has no attached items
- **THEN** the clear-all control is inoperative