# Spec Delta

## MODIFIED Requirements

### Requirement: Permission prompts
The system SHALL render a pending permission request inline in the
conversation with its available options as actionable controls, SHALL block
sending further prompts for that session until the request is answered, and
SHALL send the user's choice back through the ACP client.

The prompt SHALL identify the tool call by a human-readable name rather than
its opaque id when one is known: the title carried by the request if present,
else the title of the panel's known tool-call card for that id, else the
call's kind. If none of these is known, the raw id SHALL be shown rather than
a blank.

#### Scenario: User denies a permission request
- **WHEN** the user selects a deny option on a permission request
- **THEN** the system sends that decision to the agent and the prompt is
  replaced with its resolved state (not left pending)

#### Scenario: The prompt names the tool
- **WHEN** a permission request arrives for a tool call the panel knows by
  the title "Reading configuration file"
- **THEN** the prompt reads "Permission requested for Reading configuration
  file" (or equivalent), not the call's opaque id

#### Scenario: An unnamed call falls back to its id
- **WHEN** a permission request carries no title and no matching card is
  known
- **THEN** the prompt shows the call's id rather than an empty name