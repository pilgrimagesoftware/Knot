# Spec Delta

## ADDED Requirements

### Requirement: Knot instructions given to a launched agent

Every registration prompt the system sends an agent - the ACP protocol prompt
on a fresh session, and the deferred shell-agent prompt - SHALL carry a single
block of knot instructions, and that block SHALL meet all of the following.

It SHALL state that the agent's current task outranks a request from a
teammate, and that such a request is queued work rather than an interrupt. An
instruction that tells an agent to take on what a teammate asks without saying
what becomes of the work in hand is what let seven agents preempt one another
into a standstill; the ordering is the fix, so it is the requirement.

It SHALL decide handoff by folder rather than by project, and SHALL say
explicitly that a folder the agent shares with a teammate is the agent's to
work in. A project-ownership test is undecidable in the workspace Knot is
built for - one project, many agents - because every agent is in that project.

It SHALL name the conditions under which the agent replies to a teammate, and
SHALL forbid opening a design debate, seeking consensus, and waiting for
approval before acting. Without a bound, an answered message is an invitation
to another one.

It SHALL continue to carry the agent's knot agent ID verbatim, the name of the
MCP server the knot tools come from together with the reason that name
matters, the names `list-agents`, `send-message`, `broadcast-message` and
`set-status` each at the moment it is to be used, and the requirement to call
`set-status` before starting, on changing direction, and on finishing.

It SHALL be a single line. The shell-agent path types the prompt into a
terminal and then sends Return, so an embedded newline submits it
half-written.

It SHALL render in at most 1,000 characters. Every launched agent pays for
this text in its context window on every launch, so an instruction added here
is funded by compressing wording elsewhere, not by raising the ceiling.

#### Scenario: A teammate's request does not preempt the agent

- **WHEN** an agent is launched and reads its knot instructions
- **THEN** those instructions tell it to finish its current task before taking
  up a teammate's request, and describe that request as queued work rather
  than an interrupt

#### Scenario: A shared folder is the agent's to work in

- **WHEN** several agents are launched against the same folder
- **THEN** the knot instructions tell each of them that a folder shared with a
  teammate is theirs to work in, and reserve handing work over for a folder
  that is only the teammate's

#### Scenario: The reply loop is bounded

- **WHEN** an agent has been asked something by a teammate
- **THEN** its knot instructions name the conditions for replying and forbid
  opening a design debate, seeking consensus, or waiting for approval before
  acting

#### Scenario: The instructions stay within budget

- **WHEN** the knot instructions are rendered for any agent ID
- **THEN** the result is one line of at most 1,000 characters
