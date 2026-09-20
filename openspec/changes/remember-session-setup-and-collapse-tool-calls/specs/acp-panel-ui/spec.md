# Spec Delta

## MODIFIED Requirements

### Requirement: Tool-call rendering

The system SHALL render each tool call as a distinct card showing its kind, input summary, and result (or in-progress state) when compact mode is disabled. When compact mode is enabled, contiguous tool-call activity SHALL render as one updating summary line that reports call counts and available file or command counts. Tool execution results SHALL remain available to panel state and SHALL not be discarded by compact rendering.

#### Scenario: Default individual rendering
- **WHEN** compact mode is disabled and a tool call completes
- **THEN** the panel renders that call as its own card with its result

#### Scenario: Compact rendering
- **WHEN** compact mode is enabled and a sequence of tool calls runs
- **THEN** the panel renders one updating summary line for the sequence

#### Scenario: Failed call remains represented
- **WHEN** a tool call fails in compact mode
- **THEN** the summary reflects the failed call and its failure remains inspectable in panel state

#### Scenario: Tool call fails
- **WHEN** a tool call's result reports an error
- **THEN** the card SHALL indicate failure and show the error content instead of a normal result
