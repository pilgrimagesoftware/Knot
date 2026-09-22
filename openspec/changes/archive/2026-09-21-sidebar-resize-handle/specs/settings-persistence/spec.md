# Spec Delta

## ADDED Requirements

### Requirement: The workspace sidebar's width is a stored scalar

The settings surface SHALL hold the workspace sidebar's width as a scalar,
`sidebar_width`, defaulting to `250.0`. Writing it SHALL persist it immediately,
as with every other scalar.

It SHALL be decode-tolerant: a persisted document written before the field
existed SHALL load with it defaulted, not as an error.

A persisted value outside the sidebar's permitted range SHALL be brought into
range on load - clamped to the nearest bound - rather than rejected, loaded as
is, or reset to the default. A document hand-edited to `40` describes a
narrower sidebar than the window allows, and the nearest legal width is the
closer answer to that intent than 250 is.

A persisted value that is not a number SHALL leave the setting at its default,
as any other undecodable scalar does.

One width SHALL apply to every workspace, unlike a workspace's window bounds,
which are stored per workspace.

#### Scenario: A fresh store

- **WHEN** settings are loaded with no persisted document
- **THEN** `sidebar_width` is `250.0`

#### Scenario: A document from before the setting existed

- **WHEN** a document holding other scalars but no `sidebarWidth` is loaded
- **THEN** it loads without error and `sidebar_width` is `250.0`

#### Scenario: A persisted width is honored

- **WHEN** a document holding `sidebarWidth` `320` is loaded
- **THEN** `sidebar_width` is `320.0`

#### Scenario: A width below the minimum

- **WHEN** a document holding `sidebarWidth` `40` is loaded
- **THEN** `sidebar_width` is the minimum permitted width

#### Scenario: A width above the maximum

- **WHEN** a document holding `sidebarWidth` `5000` is loaded
- **THEN** `sidebar_width` is the maximum permitted width

#### Scenario: Writing the width persists it

- **WHEN** `sidebar_width` is set to `180.0`
- **THEN** the value is written immediately and a subsequent load reports
  `180.0`
