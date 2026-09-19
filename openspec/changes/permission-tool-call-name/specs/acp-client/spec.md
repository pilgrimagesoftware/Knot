# Spec Delta

## MODIFIED Requirements

### Requirement: Permission requests
When the agent sends a `session/request_permission` request, the system
SHALL surface it to the caller as a distinct event carrying the tool call's
id, the action's human-readable title when the agent provides one, and the
available options, and SHALL block sending the caller's decision back to the
agent until the caller responds.

#### Scenario: Session closed while a permission request is pending
- **WHEN** the caller closes the session before responding to a pending
  permission request
- **THEN** the system SHALL respond to the agent with a decline rather than
  leaving the request unanswered

#### Scenario: The request carries the tool call's title
- **WHEN** the agent's permission request includes a `title` for the tool call
- **THEN** the surfaced event includes that title alongside the call's id

#### Scenario: A request without a title is still surfaced
- **WHEN** the agent's permission request carries no tool call title
- **THEN** the event reports no title, and the request is surfaced and answer
  exactly as before