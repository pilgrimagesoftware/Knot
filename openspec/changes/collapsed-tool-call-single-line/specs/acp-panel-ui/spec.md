# Spec Delta

## ADDED Requirements

### Requirement: Collapsed tool call header is a single line
A collapsed tool call SHALL render its header on a single line: the title
SHALL truncate with an ellipsis when it does not fit rather than wrap, and
the status indicator SHALL remain visible on that same line. An expanded tool
call SHALL NOT be affected.

#### Scenario: A long title ellipsizes when collapsed
- **WHEN** a collapsed tool call's title is longer than the header's width
- **THEN** the header is one line tall and the title ends in an ellipsis

#### Scenario: The indicator stays on the line
- **WHEN** a collapsed tool call's title is ellipsized
- **THEN** the status indicator remains visible at the row's end rather than
  being pushed off or wrapped

#### Scenario: Expanded cards are unchanged
- **WHEN** the user expands a tool call whose title is long
- **THEN** the header behaves as it does today and the title is free to wrap