# Spec Delta

## Purpose

Makes the processes an agent leaves running visible and controllable from the agent's own
pane: which live processes descend from that agent's session, how long each has been running,
and a way to end one without leaving Knot. The Swift reference app has no equivalent — an
agent's spawned processes were invisible there, and this capability is new behavior in the
Rust port, not a transcription.

## ADDED Requirements

### Requirement: Every running agent exposes a session root process

A running agent SHALL expose exactly one session root process identifier, and that identifier
SHALL be the process Knot itself spawned for that agent's session: the terminal's shell
process for a shell (PTY) agent, and the ACP adapter subprocess for a panel (ACP) agent. An
agent that is not running — never started, deactivated, restarting, or whose process has
exited — SHALL expose no session root.

#### Scenario: Shell agent exposes its terminal's shell process

- **WHEN** a shell agent's terminal session is running
- **THEN** the agent's session root is the process identifier of the shell Knot spawned in
  that agent's pseudo-terminal

#### Scenario: Panel agent exposes its adapter subprocess

- **WHEN** a panel agent has a live ACP connection
- **THEN** the agent's session root is the process identifier of the adapter subprocess Knot
  spawned for that connection

#### Scenario: A stopped agent has no session root

- **WHEN** an agent is deactivated, or its session process has exited
- **THEN** the agent exposes no session root, and no process sampling is attempted for it

### Requirement: Descendant processes are enumerated transitively

Given an agent's session root, the system SHALL enumerate every live process in that root's
descendant tree, to unlimited depth, excluding the session root itself. For each descendant
the system SHALL report its process identifier, its parent's process identifier, its full
command line, and the elapsed time since it started.

A process that exits between two samples SHALL disappear from the next sample. A process
identifier that has been reused by an unrelated process SHALL NOT be reported as a descendant
unless it is genuinely a descendant of the session root at sample time.

#### Scenario: A grandchild process is reported

- **WHEN** an agent's shell runs a command that itself spawns a child, and both are alive
- **THEN** both the command and its child appear in the agent's descendant list

#### Scenario: The session root is not itself listed

- **WHEN** the descendants of an agent's session root are enumerated
- **THEN** the session root's own process does not appear in the list

#### Scenario: An exited process leaves the list

- **WHEN** a process that appeared in one sample has exited by the time of the next sample
- **THEN** it is absent from the next sample, and no error is surfaced

### Requirement: Descendants are classified as background or foreground

Each descendant SHALL carry a background/foreground classification.

For an agent whose session runs in a pseudo-terminal, a descendant SHALL be classified
foreground when it belongs to that terminal's foreground process group, and background
otherwise. For an agent whose session has no controlling terminal — the ACP adapter path —
every descendant SHALL be classified background.

When the foreground process group cannot be determined for a pseudo-terminal, every descendant
SHALL be classified background rather than reported as unknown.

#### Scenario: A command running in the terminal's foreground is foreground

- **WHEN** a shell agent is running a command in its terminal's foreground process group
- **THEN** that command is classified foreground

#### Scenario: A detached server is background

- **WHEN** a shell agent started a server that is not in the terminal's foreground process
  group
- **THEN** that server is classified background

#### Scenario: Every ACP descendant is background

- **WHEN** a panel agent's adapter has descendants
- **THEN** each of them is classified background, because the adapter has no controlling
  terminal to hold a foreground process group

### Requirement: The agent's pane presents a processes section

The selected agent's pane SHALL contain a processes section, present for both the terminal
view and the panel view. The section SHALL be collapsible and SHALL start collapsed.

Collapsed, the section SHALL show a label and the count of the agent's background descendants.
Expanded, it SHALL list one row per descendant, background descendants first, and within each
group ordered by elapsed runtime, longest-running first.

Each row SHALL show the descendant's command line on a single line, truncating with an
ellipsis rather than wrapping; its elapsed runtime; its process identifier; and its
background/foreground classification.

#### Scenario: Collapsed section shows a count

- **WHEN** an agent has three background descendants and its processes section is collapsed
- **THEN** the section shows a count of three and no rows

#### Scenario: Expanding lists the processes

- **WHEN** the user expands the processes section
- **THEN** one row per descendant is shown, background descendants before foreground ones, each
  longest-running first, and each row shows command, runtime, process identifier, and
  classification

#### Scenario: A long command line stays on one line

- **WHEN** a descendant's command line is wider than the row
- **THEN** the row renders a single line ending in an ellipsis, and the runtime, process
  identifier and actions stay visible

#### Scenario: The section appears in both views

- **WHEN** the selected agent is shown in the panel view, and again when shown in the terminal
  view
- **THEN** the processes section is present in both

### Requirement: Sampling happens off the render path and only while observed

The system SHALL NOT enumerate processes during a render pass. Samples SHALL be taken on a
recurring interval on a background task, and the rendered rows SHALL come from the most recent
completed sample.

Sampling for an agent SHALL run only while that agent's processes section is expanded and the
agent is running, and SHALL stop when the section is collapsed, the agent stops, its pane is
no longer shown, or its window closes. A collapsed section's count SHALL come from a sample
taken at the same interval; when no sample has yet completed, the count SHALL render as
unknown rather than as zero.

#### Scenario: No enumeration during a frame

- **WHEN** the agent's pane re-renders, including on every keystroke
- **THEN** no process enumeration is performed as part of that render, and the rows come from
  the last completed sample

#### Scenario: Collapsing stops the sampling

- **WHEN** the user collapses an agent's processes section
- **THEN** per-row sampling for that agent stops

#### Scenario: Stopping the agent stops the sampling

- **WHEN** a running agent with an expanded processes section is deactivated
- **THEN** sampling for it stops, and the section reports that the agent is not running

#### Scenario: Count before the first sample

- **WHEN** the processes section has been shown but no sample has completed yet
- **THEN** the count renders as unknown, not as zero

### Requirement: A listed process can be terminated

Each row SHALL offer a terminate action. Activating it SHALL first ask the user to confirm,
naming the process's command line, and SHALL do nothing if the user cancels.

On confirmation the system SHALL send the process a termination signal, wait a bounded grace
period for it to exit, and, if it is still alive when the grace period elapses, send a kill
signal. Terminating a process SHALL target only that process, never its siblings, never its
parent, and never the agent's session root.

If the process has already exited by the time the action runs, the system SHALL treat this as
success and report nothing as an error. If the signal is refused — for example because the
process is not owned by the user running Knot — the system SHALL surface a localized failure
and leave the list unchanged.

#### Scenario: Confirmed termination ends the process

- **WHEN** the user confirms terminating a listed process that ignores nothing
- **THEN** the process is sent a termination signal and is absent from the next sample

#### Scenario: A process that ignores termination is killed

- **WHEN** a process does not exit within the grace period after the termination signal
- **THEN** it is sent a kill signal

#### Scenario: Cancelling changes nothing

- **WHEN** the user opens the terminate confirmation and cancels
- **THEN** no signal is sent and the process keeps running

#### Scenario: Terminating one process leaves the others

- **WHEN** the user terminates one of an agent's several descendants
- **THEN** only that process is signalled; the agent's session root and its other descendants
  keep running

#### Scenario: Already-exited process

- **WHEN** the user confirms terminating a process that has exited since the last sample
- **THEN** no error is surfaced and the row disappears on the next sample

### Requirement: A listed process can be identified outside Knot

Each row SHALL offer actions that carry the process's identity out of Knot: copying its process
identifier, copying its full command line, and opening the platform's process viewer.

Copying SHALL place the exact value on the system clipboard, unabbreviated — the full command
line, not the truncated text the row displays.

On macOS the process-viewer action SHALL open Activity Monitor. The platform offers no
supported way to select a given process inside Activity Monitor from outside it, so the action
SHALL open the application only; the row's copy-identifier action is what carries the process
identifier across. On a platform with no known process viewer the action SHALL NOT be offered.

#### Scenario: Copying the command copies it in full

- **WHEN** the user copies the command of a row whose display text is truncated
- **THEN** the clipboard holds the complete command line, with no ellipsis

#### Scenario: Copying the identifier

- **WHEN** the user copies a row's process identifier
- **THEN** the clipboard holds that identifier

#### Scenario: Opening the process viewer on macOS

- **WHEN** the user activates the process-viewer action on macOS
- **THEN** Activity Monitor is opened

### Requirement: The section states why it is empty

When the processes section is expanded and has no rows to show, it SHALL say which case it is
in rather than render blank: the agent is not running, the agent is running and has spawned
nothing, or the last sample failed.

A failed sample SHALL NOT clear a previously shown list; the section SHALL keep showing the
last successful sample alongside the failure, and SHALL recover silently when a later sample
succeeds.

#### Scenario: Agent not running

- **WHEN** the processes section is expanded for an agent that is not running
- **THEN** it states that the agent is not running

#### Scenario: Running with nothing spawned

- **WHEN** the processes section is expanded for a running agent whose session root has no
  descendants
- **THEN** it states that the agent has spawned nothing

#### Scenario: A failed sample keeps the last list

- **WHEN** a sample fails after an earlier sample succeeded
- **THEN** the rows from the last successful sample stay shown, with the failure reported
  alongside them

#### Scenario: Recovery is silent

- **WHEN** a sample succeeds after a failed one
- **THEN** the failure notice is cleared without further user action

### Requirement: All process-section text is localized

Every user-facing string this capability introduces — the section label, the count, the column
and classification labels, each row action, the terminate confirmation, the empty states, and
the failure notices — SHALL be resolved through the application's localization lookup. A
process's own command line and identifier are data and SHALL be shown verbatim.

#### Scenario: Section text resolves through localization

- **WHEN** the processes section renders its label, count, actions, and empty states
- **THEN** each of those strings is resolved by localization key

#### Scenario: Process data is not localized

- **WHEN** a row renders a process's command line and identifier
- **THEN** they are shown exactly as the operating system reports them
