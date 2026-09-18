## Purpose

Defines the new-agent and edit-agent dialog's own contract: the fields it
offers for an agent, what they default to, and what the dialog explains to
the user about them. The sidebar's `agent-list-ui` says when this dialog
opens; this capability says what it contains.

## ADDED Requirements

### Requirement: Activation control

The dialog SHALL offer a control for the agent's activation mode, with the
two modes named as `agent-lifecycle` names them, and SHALL show the mode the
agent currently has: `passive` for a new agent, and the agent's own mode when
editing an existing one.

A hint SHALL sit next to that control saying what the two modes mean, in one
short sentence per mode. The difference is not inferable from the words
"active" and "passive" alone, and the cost of guessing wrong - an agent that
silently never starts, or a workspace that launches eight subprocesses -
falls on the user.

Submitting the dialog SHALL apply the chosen mode to the agent. Changing only
the activation mode SHALL NOT restart a running agent, per `agent-lifecycle`.

#### Scenario: A new agent offers passive

- **WHEN** the user opens the dialog to create an agent
- **THEN** the activation control shows `passive`, and the hint explains both
  modes

#### Scenario: Editing shows the agent's current mode

- **WHEN** the user opens the dialog on an existing `active` agent
- **THEN** the activation control shows `active`

#### Scenario: Changing the mode of a running agent does not disturb it

- **WHEN** the user edits a running agent, changes only its activation mode,
  and submits
- **THEN** the agent's mode is updated and the agent keeps running
