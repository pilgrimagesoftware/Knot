# Spec Delta

## ADDED Requirements

### Requirement: Permission decisions resolve by option kind

Each permission option SHALL be surfaced with its `kind` (`allow_once`,
`allow_always`, `reject_once`, `reject_always`) when the agent sends a known
one. An unknown or malformed kind SHALL leave that option without a kind,
and SHALL NOT discard the request's other options.

A decision SHALL be answered with the option of its own kind, never by the
options' order or wording:

- Allow SHALL answer with the `allow_once` option.
- Always Allow SHALL answer with the `allow_always` option.
- Deny SHALL answer with the `reject_once` option, else the `reject_always`
  option.
- A direct choice SHALL answer with the chosen option.

A decision whose kind the request does not offer SHALL NOT be answered with
some other option.

When no option carries a kind, Allow SHALL answer with the first option and
Deny with the first option whose id or name reads as a refusal, as before
kinds were read.

#### Scenario: Always Allow listed first

- **WHEN** an agent offers Always Allow, Allow and Reject, in that order,
  and the user allows
- **THEN** the agent receives the Allow option's id, not Always Allow's

#### Scenario: Always Allow chosen

- **WHEN** the user chooses Always Allow on that request
- **THEN** the agent receives the Always Allow option's id

#### Scenario: Deny on a Reject option

- **WHEN** the agent's refusal option is named "Reject" with kind
  `reject_once`, and the user denies
- **THEN** the agent receives that option's id

#### Scenario: An unknown kind

- **WHEN** one option carries a kind this client does not know
- **THEN** the request still offers every option, that one without a kind
