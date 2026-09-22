# mcp-messaging Specification

## Purpose
Defines the in-process agent-to-agent message queue: send, broadcast, check,
unread tracking, the workspace and companion routing rules that constrain who
may message whom, the idle-time delivery nudge, and retention cleanup. Messages
are runtime state; the intended Rust behavior matches the Swift reference,
including that messages do not survive an app restart.

## Requirements

### Requirement: Message model

A message SHALL carry a unique id, sender id, recipient id (both stored as
agent uuids), content, a timestamp, and a read flag defaulting to false.
Messages SHALL be held in memory only and SHALL NOT persist across app
restarts.

#### Scenario: New message is unread

- **WHEN** a message is created
- **THEN** its read flag is false and it has a fresh id and timestamp

### Requirement: Sender must be registered

Send and broadcast SHALL be rejected unless the sender resolves to a known
agent that is registered with MCP. Rejection SHALL return an explanatory
string; broadcast SHALL return a recipient count of zero.

#### Scenario: Unregistered sender

- **WHEN** an unregistered agent calls send-message
- **THEN** no message is stored and the caller receives "Sender not registered"

### Requirement: Recipients are workspace-scoped

A message SHALL be delivered only to a recipient in the same workspace as the
sender. A recipient outside the sender's workspace SHALL be treated as not
found.

#### Scenario: Cross-workspace send fails

- **WHEN** the sender targets an agent in another workspace
- **THEN** the result is "Recipient not found" and nothing is stored

### Requirement: Shell agents cannot receive messages

Send and broadcast SHALL skip shell agents. A direct send to a shell agent
SHALL return "Cannot send messages to shell agents".

#### Scenario: Direct send to shell agent

- **WHEN** an agent sends to a shell agent
- **THEN** the send is rejected with the shell-agent message

### Requirement: Companion routing rules

A companion agent SHALL only send to, and only receive from, its owner. A
non-owner sending to a companion SHALL be rejected with "Only the owner can
send messages to a companion agent"; a companion sending to anyone other than
its owner SHALL be rejected with "Companion agents can only send messages to
their owner". Broadcast SHALL apply the same filter per recipient.

#### Scenario: Owner messages its companion

- **WHEN** owner `A` sends to companion `C` where `C.created_by == A`
- **THEN** the message is stored

#### Scenario: Third party messages a companion

- **WHEN** agent `B` (not the owner) sends to companion `C`
- **THEN** the send is rejected

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

### Requirement: Check and mark-read

Check-messages SHALL return the caller's unread messages and, by default, mark
them read. A caller MAY request a non-destructive read that leaves the flags
unchanged. Unread queries SHALL report whether any unread message exists and
SHALL be able to return the most recent unread message id.

#### Scenario: Check clears unread

- **WHEN** an agent checks messages with default options
- **THEN** it receives its unread messages and a subsequent unread query
  reports none

### Requirement: Broadcast fan-out

Broadcast SHALL create one message per eligible recipient in the sender's
workspace (excluding the sender, unregistered agents, shell agents, and
companion-rule violations). Each stored message SHALL follow the same
idle-gated delivery nudge as a direct send (see "Idle-time delivery nudge").
The return value SHALL be the number of messages created.

#### Scenario: Broadcast to a mixed workspace

- **WHEN** a registered agent broadcasts in a workspace with two other
  registered non-shell agents and one shell agent
- **THEN** two messages are created and the count is 2

### Requirement: Retention cleanup

The system SHALL cap stored read messages, discarding the oldest read messages
beyond a fixed retention (100). Unread messages SHALL never be discarded by
cleanup.

#### Scenario: Old read messages pruned

- **WHEN** more than 100 read messages accumulate
- **THEN** the oldest read messages are removed down to 100 and no unread
  message is touched

### Requirement: Inbox nudges preserve interrupted session work

An automatic inbox nudge SHALL be queued behind the agent's active session and
SHALL NOT interrupt, replace, discard, or permanently suspend the currently
pending task. The nudge text SHALL tell the agent to check its inbox and, if
there is nothing to do, continue its previous work. After the nudge is
acknowledged or handled, the agent session SHALL continue the pending task
from its prior state.

#### Scenario: Nudge arrives while a task is paused

- **WHEN** an agent has pending work and receives an automatic "check your
  inbox" nudge
- **THEN** the nudge waits behind the pending work, and its text tells the
  agent to continue the previous work if there is nothing to do

#### Scenario: Nudge interrupts delivery of a task update

- **WHEN** an inbox nudge arrives while the agent session is delivering an
  update for its current task
- **THEN** the nudge is queued without interrupting the update, and the update
  state and current task resume afterward

#### Scenario: Ordinary user message remains independent

- **WHEN** a user sends a normal message rather than an automatic inbox nudge
- **THEN** the existing prompt and queue rules apply without continuation
  semantics being inferred
