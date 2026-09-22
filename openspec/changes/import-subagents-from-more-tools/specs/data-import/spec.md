## Purpose

Extends the `data-import` capability with a source-and-format requirement per
coding-agent tool whose subagent format has been confirmed against a real
installation, and narrows the provider-registry requirement as each reader
lands.

## MODIFIED Requirements

### Requirement: Subagent definition provider registry

The system SHALL resolve a subagent-definition provider by coding-agent tool.
The registry SHALL know the tools `claude`, `codex`, `opencode` and `gemini`.
For a tool the registry does not know, and for one it knows but has no reader
for, subagent import SHALL be reported as unsupported and SHALL be a no-op.

A provider SHALL read only; it SHALL NOT create, modify or delete anything in
the tool it reads from.

The import surface SHALL offer only the tools that have a reader. A tool
whose format is not yet confirmed SHALL be absent from it rather than listed
and empty.

A tool's reader SHALL be written only from a format read off a populated
installation of that tool. A tool confirmed to have no subagent concept SHALL
be removed from the registry rather than carried as a reader that can never
find anything.

#### Scenario: Unsupported tool

- **WHEN** subagent definitions are requested for a tool with no provider
- **THEN** support is reported false and no read is attempted

#### Scenario: A tool with no confirmed format is not offered

- **WHEN** the user opens the import surface
- **THEN** only tools with a reader are listed, and a tool whose format is
  unconfirmed is absent rather than shown holding nothing

#### Scenario: A reader lands without a UI change

- **WHEN** a provider is implemented for a tool the registry already knows
- **THEN** that tool appears in the import surface with no change to the
  surface itself
