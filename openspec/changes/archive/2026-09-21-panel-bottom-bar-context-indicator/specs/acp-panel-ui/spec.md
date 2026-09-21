# Spec Delta

## ADDED Requirements

### Requirement: Input area context indicator
The input area SHALL display a radial indicator in its bottom bar when the ACP
session reports a positive context-window size. Its fill SHALL represent used
context tokens as a fraction of the reported total; its remainder SHALL
represent available context. Its tooltip SHALL state the used and total token
counts. The indicator SHALL be absent until the agent reports usable values.

#### Scenario: A usage update fills the indicator
- **WHEN** an ACP session reports 53,000 used tokens from a 200,000-token
  context window
- **THEN** the radial indicator is filled to 26.5 percent and its tooltip
  identifies both values

#### Scenario: No usage update is available
- **WHEN** the ACP agent does not report context-window usage
- **THEN** the bottom bar does not show a context indicator
