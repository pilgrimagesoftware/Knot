# Spec Delta

## Purpose

Provides a compact, optional view of tool activity that keeps agent conversations readable while retaining meaningful progress and result information.

## ADDED Requirements

### Requirement: Tool calls can use one live summary line

When compact tool-call mode is enabled, contiguous tool-call activity in a turn SHALL render as one updating summary line instead of separate visible tool-call cells.

#### Scenario: Aggregate tool activity
- **WHEN** three tool calls run consecutively
- **THEN** the panel shows one summary line whose counts reflect all three calls

### Requirement: Summary reports useful activity

The summary line SHALL report the number of tool calls and SHALL include available activity counts such as files edited, files read, or commands run. Counts SHALL update as events arrive.

#### Scenario: Update after a file edit
- **WHEN** a tool call edits a file while the summary is visible
- **THEN** the summary updates to include the new edited-file count

### Requirement: Summary boundaries are explicit

The compact summary SHALL end when the next prompt or non-tool output begins. Tool calls after that boundary SHALL start a new summary line.

#### Scenario: Assistant text separates summaries
- **WHEN** tool calls are followed by assistant text and then another tool call
- **THEN** the later tool call is represented by a new summary line

### Requirement: Normal rendering remains available

The system SHALL default to existing individual tool-call rendering unless compact mode is enabled, and the user SHALL be able to switch modes without changing tool execution or losing tool-call details.

#### Scenario: Disable compact mode
- **WHEN** the user turns compact mode off
- **THEN** subsequent tool calls render as individual cells
