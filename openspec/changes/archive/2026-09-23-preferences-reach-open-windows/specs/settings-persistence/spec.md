# Spec Delta

## ADDED Requirements

### Requirement: A preference change applies to open windows

A preference written through the settings surface SHALL take effect in every
already-open window that draws from it, without that window being closed and
reopened.

A window MAY hold its own copy of the settings surface. If it does, the copy
SHALL be refreshed when preferences are written, and the refresh SHALL NOT be
performed on the render path.

Refreshing preferences SHALL replace only the scalar preferences. The durable
collections - saved agents, saved workspaces, workspace UI state, personas,
bench agents, recent repos and recorded pull requests - SHALL be left as the
refreshing window holds them, so a refresh cannot discard roster state the
window has not yet written.

#### Scenario: Compact tool calls applies to an open panel

- **WHEN** a workspace window is open and the user enables compact tool-call
  mode in the settings window
- **THEN** that window's panel draws the summary line without being reopened

#### Scenario: Refresh keeps the roster

- **WHEN** a window whose in-memory roster differs from the preferences
  document refreshes its preferences
- **THEN** its scalar preferences match the document and its roster is
  unchanged

#### Scenario: A window opened later is unaffected

- **WHEN** a preference is changed and a workspace window is opened afterwards
- **THEN** that window reads the changed value, as it did before
