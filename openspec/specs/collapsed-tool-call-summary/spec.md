# collapsed-tool-call-summary Specification

## Purpose
Provides a compact, optional view of tool activity that keeps agent
conversations readable while retaining meaningful progress and result
information. Off by default; the per-call cards described in `acp-panel-ui`
remain the standard presentation.

## Requirements

### Requirement: Tool calls can use one live summary line

When compact tool-call mode is enabled, contiguous tool-call activity in a
turn SHALL render as one updating summary line instead of separate visible
tool-call cells.

#### Scenario: Aggregate tool activity
- **WHEN** three tool calls run consecutively
- **THEN** the panel shows one summary line whose counts reflect all three
  calls

### Requirement: Summary reports useful activity

The summary line SHALL report the number of tool calls and SHALL include
available activity counts such as files edited, files read, or commands run.
Counts SHALL update as events arrive.

A count the tool metadata does not support SHALL be omitted rather than
guessed at or shown as zero. The call count is always available and SHALL
always be reported. A file edited more than once in one run SHALL count once.

#### Scenario: Update after a file edit
- **WHEN** a tool call edits a file while the summary is visible
- **THEN** the summary updates to include the new edited-file count

### Requirement: Summary boundaries are explicit

The compact summary SHALL end when the next prompt or non-tool output begins.
Tool calls after that boundary SHALL start a new summary line.

#### Scenario: Assistant text separates summaries
- **WHEN** tool calls are followed by assistant text and then another tool
  call
- **THEN** the later tool call is represented by a new summary line

### Requirement: Failures stay visible

A failed tool call SHALL be reflected in its run's summary line, and its
record SHALL remain available for inspection.

#### Scenario: Failed call remains represented
- **WHEN** a tool call fails while compact mode is enabled
- **THEN** the summary reports the failure and the call's own record is still
  reachable

### Requirement: Normal rendering remains available

The system SHALL default to existing individual tool-call rendering unless
compact mode is enabled, and the user SHALL be able to switch modes without
changing tool execution or losing tool-call details.

Switching modes SHALL NOT alter stored conversation state: both modes SHALL
present the same underlying calls.

#### Scenario: Disable compact mode
- **WHEN** the user turns compact mode off
- **THEN** subsequent tool calls render as individual cells

#### Scenario: Inspect a summarized run
- **WHEN** the user opens a run that is drawn as a summary line
- **THEN** that run's tool calls render as individual cells again
