# panel-shell-passthrough Specification

## Purpose
Lets the user run a shell command from the panel prompt by prefixing it with
`!`, see its output in the conversation, and hand that output to the agent as
context on the next prompt — without leaving the panel or spending an agent
turn. The Swift reference app has no equivalent; this capability is new to the
Rust port.

## Requirements

### Requirement: A leading `!` marks the prompt as a shell command

When the prompt buffer's first character is `!` and at least one
non-whitespace character follows it, submitting the prompt SHALL run the text
after the `!` as a shell command instead of sending the buffer to the agent.
The buffer SHALL clear on submission as it does for an ordinary prompt.

A buffer whose first character is not `!` SHALL be an ordinary prompt, so a
single leading space is the escape for a prompt that must begin with a literal
`!`. That leading whitespace SHALL be trimmed before the prompt is sent, and
the system SHALL NOT otherwise alter the text it sends.

A buffer of `!` alone, or `!` followed only by whitespace, SHALL NOT be
submitted: it is neither a command nor a message.

#### Scenario: A prefixed line runs as a command
- **WHEN** the user submits a prompt whose buffer is `!ls -la`
- **THEN** `ls -la` runs as a shell command, the buffer clears, and nothing is
  sent to the agent

#### Scenario: A leading space sends the text to the agent
- **WHEN** the user submits a prompt whose buffer is ` !important, read this`
- **THEN** the text reaches the agent as `!important, read this` and no
  command runs

#### Scenario: A bare bang does nothing
- **WHEN** the user submits a prompt whose buffer is `!` or `!   `
- **THEN** no command runs, nothing is sent to the agent, and the buffer is
  left as the user typed it

#### Scenario: A multi-line command runs whole
- **WHEN** the user submits a buffer beginning with `!` and spanning several
  lines
- **THEN** every line after the `!` is passed to the shell as one command

### Requirement: Commands run non-interactively in the agent's folder

A shell command SHALL run through the user's configured login shell, with the
selected agent's folder as its working directory, and with the environment the
application itself runs under. It SHALL run without a terminal: no TTY is
allocated and standard input is closed, so a command that waits for input
reads end-of-file rather than hanging.

Each panel runs its commands independently. A command SHALL NOT observe or
inherit state — shell variables, directory changes, job control — from a
command run earlier in the same panel or in another panel.

#### Scenario: The command runs where the agent works
- **WHEN** the user runs `!pwd` in a panel whose agent's folder is a worktree
- **THEN** the output is that worktree's path

#### Scenario: A command that reads input does not hang
- **WHEN** the user runs a command that reads from standard input
- **THEN** the read reports end-of-file and the command finishes rather than
  waiting

#### Scenario: Directory changes do not persist
- **WHEN** the user runs `!cd ..` and then `!pwd`
- **THEN** the second command reports the agent's folder, unchanged by the
  first

#### Scenario: The shell cannot be started
- **WHEN** the shell cannot be launched
- **THEN** the failure is reported in the command's own conversation entry and
  no other panel state changes

### Requirement: Execution is independent of the agent session

A shell command SHALL run as soon as it is submitted, whatever state the ACP
session is in. It SHALL NOT join the prompt queue, SHALL NOT interrupt or
cancel an active turn, and SHALL NOT be held by a pending permission request.
Running a command SHALL NOT start, stop, or otherwise disturb the agent
session.

A panel MAY have several commands running at once; each keeps its own entry
and completes on its own.

#### Scenario: A command runs during a turn
- **WHEN** the user submits `!git status` while the agent is mid-response
- **THEN** the command runs immediately, its entry appears in the
  conversation, and the turn continues uninterrupted

#### Scenario: A pending permission request does not hold a command
- **WHEN** a permission request is awaiting an answer and the user submits a
  shell command
- **THEN** the command runs and the permission request is still pending

#### Scenario: A command does not enter the queue
- **WHEN** the user submits a shell command while prompts are queued
- **THEN** the queue is unchanged and no queued prompt is delivered on the
  command's account

### Requirement: Output and exit status are captured and bounded

The system SHALL capture the command's standard output and standard error and
SHALL show them as they are produced rather than only on completion. It SHALL
record the command's exit status, distinguishing success from failure and from
termination by a signal.

Capture SHALL be bounded in both size and time:

- Once captured output reaches a fixed size limit, further output SHALL be
  discarded and the entry SHALL say that the output was truncated. The command
  SHALL be allowed to continue.
- A command still running at a fixed wall-clock limit SHALL be terminated, and
  the entry SHALL say that it timed out and SHALL keep whatever output was
  captured.

Neither limit SHALL be reachable in a way that blocks the conversation from
rendering while the command runs.

#### Scenario: Output appears while the command runs
- **WHEN** a command emits output over several seconds
- **THEN** the entry shows the output as it arrives, and the conversation
  stays interactive throughout

#### Scenario: A failing command reports its status
- **WHEN** a command exits non-zero
- **THEN** the entry shows the exit status as a failure alongside the captured
  output

#### Scenario: Runaway output is truncated
- **WHEN** a command produces more output than the size limit
- **THEN** the entry holds output up to the limit and states that it was
  truncated

#### Scenario: A command that never ends is stopped
- **WHEN** a command is still running at the wall-clock limit
- **THEN** it is terminated and its entry states that it timed out

### Requirement: A running command can be cancelled

While a command is running, its conversation entry SHALL offer a control that
terminates it. Cancelling SHALL leave the entry in place, marked cancelled,
with the output captured so far. A cancelled command's result SHALL NOT be
attached to any later prompt.

Cancelling SHALL affect only that command: other running commands and the
agent turn are untouched.

#### Scenario: The user stops a long command
- **WHEN** the user activates the cancel control on a running command
- **THEN** the command is terminated, its entry reads as cancelled, and the
  output captured up to that point remains visible

#### Scenario: Cancelling is scoped to one command
- **WHEN** two commands are running and the user cancels one
- **THEN** the other continues and the agent turn, if any, is unaffected

#### Scenario: A cancelled result is not shared
- **WHEN** the user cancels a command and then sends an ordinary prompt
- **THEN** the cancelled command's output is not attached to that prompt

### Requirement: A shell command renders as its own conversation entry

Submitting a shell command SHALL append an entry to the conversation at the
point it was submitted. The entry SHALL show the command as typed, the folder
it runs in, its output, and its state — running, finished with an exit status,
cancelled, timed out, or failed to start. Output SHALL be rendered
monospaced and SHALL preserve the line structure the command produced.

The entry SHALL be visually distinct from a user message, an assistant
message, and a tool call, so the reader can tell at a glance that the user ran
it rather than the agent.

The entry SHALL state whether its result is still pending delivery to the
agent or has already been shared, and SHALL offer a control that discards a
pending result.

#### Scenario: The entry records what ran
- **WHEN** a command finishes
- **THEN** its entry shows the command, the folder, the captured output, and
  the exit status

#### Scenario: Pending results are marked
- **WHEN** a command finishes successfully and no prompt has been sent since
- **THEN** its entry says the result is waiting to be shared with the agent

#### Scenario: A pending result can be discarded
- **WHEN** the user discards a pending result
- **THEN** the entry remains in the conversation, no longer marked pending,
  and the result is not attached to any later prompt

### Requirement: Results are attached to the next prompt

When the user sends an ordinary prompt, every pending shell result in that
panel SHALL be attached to it, in the order the commands were submitted, so
the agent receives each command and its captured output alongside the user's
text. Attached results SHALL then be marked as shared and SHALL NOT be
attached again.

A result SHALL be pending only if its command finished on its own — a
cancelled command, a command that failed to start, and a discarded result are
never attached. A command still running when the prompt is sent SHALL stay
pending and be attached to a later prompt instead.

Pending results SHALL NOT be sent to the agent on their own: if no prompt
follows, the agent never receives them. Pending results SHALL be scoped to the
panel they ran in and SHALL NOT reach another agent.

#### Scenario: A command's output reaches the agent with the next prompt
- **WHEN** the user runs `!git status`, then sends "what changed?"
- **THEN** the agent receives the prompt together with the command and its
  output, and the command's entry is marked shared

#### Scenario: Several results attach in order
- **WHEN** the user runs two commands and then sends a prompt
- **THEN** both results are attached in the order they were submitted, and
  neither is attached to a subsequent prompt

#### Scenario: An unfinished command waits for a later prompt
- **WHEN** the user sends a prompt while a command is still running
- **THEN** that command's result is not attached to that prompt and remains
  pending

#### Scenario: Nothing is sent without a prompt
- **WHEN** the user runs commands and sends no prompt
- **THEN** the agent receives nothing

#### Scenario: Results do not cross panels
- **WHEN** the user runs a command in one panel and sends a prompt in another
- **THEN** the second panel's prompt carries no result from the first

### Requirement: The composer shows that a line will run

While the prompt buffer is recognised as a shell command, the input area SHALL
indicate that submitting will run a command rather than message the agent. The
indication SHALL appear and disappear as the buffer is edited, and SHALL NOT
alter the buffer's text.

#### Scenario: Typing the trigger marks the input
- **WHEN** the user types `!` as the first character of an empty buffer
- **THEN** the input area shows that the line will run as a shell command

#### Scenario: Editing away the trigger clears the mark
- **WHEN** the user removes the leading `!` or types a character before it
- **THEN** the indication disappears and the prompt reads as an ordinary
  message again
