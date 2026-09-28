# Spec Delta

## Purpose

Lets the user hand an issue or an OpenSpec change to one of the workspace's
agents from the Changes view, by sending that agent a prompt naming it.

## ADDED Requirements

### Requirement: Issue and change rows offer a Send prompt to submenu

The context menu and the actions menu of every issue row and every change row
SHALL hold a "Send prompt to" item opening a submenu that lists the
workspace's agents in alphabetical order by name. An agent that cannot receive
a prompt - one that is not running - SHALL be listed disabled rather than
left out, so the list does not change shape as agents start and stop. A
workspace with no agents SHALL show the item disabled.

The Swift reference has no such menu.

#### Scenario: The submenu lists the agents

- **WHEN** the workspace has agents Bo and Ada, both running, and the user
  opens an issue row's context menu and hovers "Send prompt to"
- **THEN** the submenu lists Ada, then Bo

#### Scenario: A stopped agent is disabled

- **WHEN** agent Ada is not running
- **THEN** Ada is listed in the submenu and cannot be chosen

#### Scenario: The actions menu holds the same submenu

- **WHEN** the user opens a change row's actions menu
- **THEN** it holds a "Send prompt to" submenu listing the same agents

### Requirement: Choosing an agent sends it a prompt naming the item

Choosing an agent in the submenu SHALL send that agent, and no other, a single
prompt:

- for an issue: `Work on this issue: <issue URL>`
- for an OpenSpec change: `Work on this OpenSpec change: <change name>`

The prompt SHALL be delivered the way a prompt the user typed into that agent
is delivered: an idle agent SHALL start working on it, and an agent in the
middle of a turn SHALL queue it, shown in its queue like any other queued
prompt. The prompt text SHALL be a localized string with the URL or name
substituted.

Sending SHALL NOT change the window's view: the Changes view stays shown. The
target agent's sidebar row SHALL reflect that it is working, as any prompt
makes it do.

#### Scenario: Sending an issue

- **WHEN** the user chooses Ada under "Send prompt to" on the row for
  `https://github.com/acme/widget/issues/42`
- **THEN** Ada receives the prompt
  `Work on this issue: https://github.com/acme/widget/issues/42`
- **AND** no other agent receives anything

#### Scenario: Sending a change

- **WHEN** the user chooses Bo under "Send prompt to" on the `add-login` row
- **THEN** Bo receives the prompt `Work on this OpenSpec change: add-login`

#### Scenario: A busy agent queues the prompt

- **WHEN** Ada is in the middle of a turn and the user sends Ada an issue
- **THEN** the prompt appears in Ada's queue and is delivered when the turn ends

#### Scenario: The view stays put

- **WHEN** the user sends an issue to an agent
- **THEN** the Changes view is still shown on the Issues tab
