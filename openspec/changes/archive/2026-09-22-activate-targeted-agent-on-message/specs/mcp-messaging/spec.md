# Spec Delta

## MODIFIED Requirements

### Requirement: Idle-time delivery nudge

When a message is stored (whether from a direct send or a broadcast) and the
recipient is currently Idle, the system SHALL deliver a short "check your
inbox" prompt to the recipient over whichever channel drives it: a prompt in
its session for an agent running over ACP. When the recipient is not Idle,
the message waits in the queue and is surfaced on the recipient's next
transition to Idle.

The nudge SHALL be subject to a guard that prevents it landing on an agent
that is mid-conversation: it SHALL NOT be delivered while the recipient has a
turn in flight or a permission request outstanding, since a prompt sent then
is either rejected or jumps the queue ahead of the work the agent is already
doing.

A recipient SHALL be nudged at most once per message. A message that has
already produced a nudge SHALL NOT produce another while it stays unread, so
an agent that leaves its inbox unread is not prompted on every idle moment.

Divergence from the Swift reference: the Swift app idle-gates the nudge for
direct sends but nudges every broadcast recipient unconditionally. The Rust
port SHALL idle-gate both paths identically.

A recipient with no live session gets one this way only when a direct send
activated it (see "Direct send activates a deactivated recipient"); the nudge
itself SHALL NOT start a session. Activation and the nudge are separate
effects of the same send - activation makes the recipient's session exist,
the nudge (subject to its own idle/turn-in-flight guard, evaluated against
the recipient's state after activation) is what tells it a message is
waiting.

#### Scenario: Recipient idle at send time

- **WHEN** a message arrives for an Idle recipient with the guard inactive
- **THEN** the inbox prompt is delivered to that recipient

#### Scenario: Recipient busy at send time

- **WHEN** a message arrives for a Working recipient
- **THEN** nothing is delivered and the message stays unread until the
  recipient is Idle

#### Scenario: Broadcast to a busy recipient

- **WHEN** a broadcast reaches an eligible recipient that is currently Working
- **THEN** the message is stored unread and no prompt is delivered until that
  recipient next becomes Idle

#### Scenario: Recipient mid-turn is not interrupted

- **WHEN** a message arrives for a recipient whose session has a turn in
  flight or a permission request outstanding
- **THEN** no prompt is delivered, and it is delivered once that turn ends

#### Scenario: An unread message nudges once, not repeatedly

- **WHEN** a recipient is nudged about a message and leaves it unread while
  going idle again
- **THEN** no further prompt is delivered for that message

#### Scenario: Recipient with no live session

- **WHEN** a message arrives for an agent whose session is not running
- **THEN** nothing is delivered, the message stays unread, and it is
  delivered when that agent next has a live session and is idle

#### Scenario: Broadcast recipient with no live session stays queued

- **WHEN** a broadcast reaches an eligible recipient whose session is not
  running
- **THEN** nothing is delivered, the message stays unread, no activation is
  triggered, and it is delivered when that agent next has a live session and
  is idle

## ADDED Requirements

### Requirement: Direct send activates a deactivated recipient

A direct `send` whose recipient resolves to a deactivated agent SHALL
activate that agent - starting its session the same way selecting it in the
UI does - as part of delivering the message. Activation SHALL happen only
after the existing workspace, shell-agent, and companion routing checks
pass; a send rejected by any of those SHALL NOT activate anyone.

`broadcast` SHALL NOT activate any recipient: it addresses every eligible
member of the workspace rather than one agent the sender specifically chose,
so a deactivated agent stays stopped until a direct send targets it or a
person selects it.

#### Scenario: Direct send to a deactivated recipient starts it

- **WHEN** a registered agent sends directly to a deactivated agent in its
  workspace
- **THEN** the message is stored and the recipient's session starts

#### Scenario: A rejected send does not activate anyone

- **WHEN** a direct send to a deactivated agent is rejected by a routing rule
  (cross-workspace, shell agent, or companion violation)
- **THEN** the recipient stays deactivated

#### Scenario: Broadcast never activates a deactivated recipient

- **WHEN** a broadcast reaches a deactivated agent in the sender's workspace
- **THEN** the message is stored unread and the recipient stays deactivated
