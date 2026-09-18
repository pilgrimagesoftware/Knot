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
