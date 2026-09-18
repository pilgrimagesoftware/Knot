## ADDED Requirements

### Requirement: Deactivate from the context menu

The agent row's context menu SHALL offer Deactivate for an agent that is
currently running, in the group that holds Restart Agent and Remove Agent and
immediately before Restart Agent. Selecting it SHALL deactivate that agent per
`agent-lifecycle`'s deactivation requirement.

Deactivate SHALL be absent for an agent that is not running - there is nothing
to stop - rather than shown disabled, matching how this menu hides every other
item that does not apply.

Deactivate SHALL NOT ask for confirmation. Nothing is lost that the agent
cannot get back by being selected again, which is the test this menu applies
to Restart and Remove and which those two fail.

#### Scenario: Deactivate a running agent from its row

- **WHEN** the user right-clicks a running agent and selects Deactivate
- **THEN** the agent is deactivated, with no confirmation prompt

#### Scenario: A stopped agent has nothing to deactivate

- **WHEN** the user right-clicks a `passive` agent that has never been
  selected, or one that is already deactivated
- **THEN** Deactivate is absent from the menu

#### Scenario: Deactivate sits with the other session actions

- **WHEN** the menu is opened on a running non-companion agent
- **THEN** Deactivate appears immediately above Restart Agent, in the same
  group as Restart Agent and Remove Agent

### Requirement: A stopped agent is distinguishable in the sidebar

An agent that is not running SHALL be visually distinguishable in the
sidebar from one that is, so a workspace of mixed agents can be read at a
glance without opening each row.

The distinction SHALL apply to any agent that is not running, whether it is
passive and never started or was deactivated - what the user needs to know
is which agents are live, not why each one is not.

It SHALL NOT rely on the agent's state dot, which reports what a *running*
agent is doing (idle, working, awaiting input, error) and has no value that
means "not running at all".

#### Scenario: A passive agent that has never started

- **WHEN** a workspace holds a running agent and a passive agent that has
  not been selected
- **THEN** the two rows are visually distinguishable, and the passive one
  reads as not running

#### Scenario: Activating a passive agent brings its row up to the others

- **WHEN** the user selects that passive agent and it starts
- **THEN** its row becomes indistinguishable from any other running agent's

#### Scenario: A deactivated agent reads the same as one never started

- **WHEN** a running agent is deactivated
- **THEN** its row takes the same not-running appearance as a passive agent
  that never started
