# Spec Delta

## MODIFIED Requirements

### Requirement: Input area permission mode selector
The input area SHALL provide a selector for the agent's permission mode,
applied to the next message and subsequent turns until changed.

The system SHALL locate the agent's permission-mode option without
requiring the agent to have categorized it. A declared option SHALL be
matched on its category when it carries one, and otherwise on its
identifier or name. Where several options match, the one the agent listed
first SHALL win. An option the system cannot render as a picker SHALL be
ignored.

#### Scenario: Change permission mode
- **WHEN** the user selects a different permission mode from the selector
- **THEN** subsequent agent turns run under the newly selected permission
  mode

#### Scenario: Uncategorized option still populates the selector
- **WHEN** the agent declares a selectable permission-mode option but
  attaches no category to it
- **THEN** the selector is populated from that option, rather than showing
  the "doesn't report permission modes" empty state

#### Scenario: Unknown category falls back to the option's own names
- **WHEN** the agent declares a selectable permission-mode option under a
  category the system does not recognize
- **THEN** the option is still matched by its identifier or name, and the
  selector is populated from it

#### Scenario: The agent's ordering breaks a tie
- **WHEN** more than one declared option matches the permission-mode
  selector
- **THEN** the selector uses the one the agent listed first

#### Scenario: No matching option shows the empty state
- **WHEN** the agent declares no option matching the permission-mode
  selector by category, identifier or name
- **THEN** the selector shows its "doesn't report permission modes" empty
  state, as before

### Requirement: Input area model selector
The input area SHALL provide a selector for which model the agent uses,
applied starting with the next message.

The system SHALL locate the agent's model option under the same matching
rule as the permission-mode selector: category first, then identifier or
name, ties broken by the agent's ordering, unrenderable options ignored.

#### Scenario: Change model
- **WHEN** the user selects a different model from the selector
- **THEN** the next message is sent using the newly selected model

#### Scenario: Uncategorized option still populates the selector
- **WHEN** the agent declares a selectable model option but attaches no
  category to it
- **THEN** the selector is populated from that option, rather than showing
  the "doesn't report selectable models" empty state

### Requirement: Input area effort selector
The input area SHALL provide a selector for the agent's reasoning effort
level, applied starting with the next message.

The system SHALL locate the agent's effort option under the same matching
rule as the permission-mode selector, and SHALL recognize the protocol's
own category for reasoning level in addition to the identifier spellings
it already accepts.

#### Scenario: Change effort level
- **WHEN** the user selects a different effort level from the selector
- **THEN** the next message is sent using the newly selected effort level

#### Scenario: The protocol's reasoning category is recognized
- **WHEN** the agent declares its reasoning-level option under the
  protocol's standard category for that concept
- **THEN** the effort selector is populated from that option

#### Scenario: Uncategorized option still populates the selector
- **WHEN** the agent declares a selectable effort option but attaches no
  category to it
- **THEN** the selector is populated from that option, rather than showing
  the "doesn't report selectable effort levels" empty state
