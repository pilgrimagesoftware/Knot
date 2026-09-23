# agent-processes Specification

## Purpose

Makes the processes an agent leaves running visible and controllable from the agent's own
pane: which live processes descend from that agent's session, how long each has been running,
and a way to end one without leaving Knot. The Swift reference app has no equivalent — an
agent's spawned processes were invisible there, and this capability is new behavior in the
Rust port, not a transcription.

## Requirements

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

The header SHALL carry a summary of what the agent has running, which SHALL differ by
disclosure state:

- **Collapsed**, it SHALL name the running processes rather than count them: the distinct
  process names, deduplicated, in the order the list already carries. Where more processes are
  running than the header lists names for, it SHALL carry a remainder counting the processes
  the listed names do not cover.
- **Expanded**, it SHALL give the number of descendants listed below it. The rows are already
  the detail, so the header does not repeat them.

Where nothing is running — no descendants, or an agent that is not running at all — the
summary SHALL say so in words. It SHALL NOT be blank, and it SHALL NOT be the unknown marker.
An agent that is not running SHALL read as nothing running whatever its last sample held, so
the header cannot contradict the body's "not running".

The unknown marker SHALL be shown only while it is true: the section is shown and its first
sample has not completed.

Expanded, the section SHALL list one row per descendant, background descendants first, and
within each group ordered by elapsed runtime, longest-running first.

Each row SHALL show the descendant's command line on a single line, truncating with an
ellipsis rather than wrapping; its elapsed runtime; its process identifier; and its
background/foreground classification.

#### Scenario: Collapsed section names what is running

- **WHEN** an agent is running `node` and `rg` and its processes section is collapsed
- **THEN** the header names both and shows no rows

#### Scenario: Repeats of one name are folded together

- **WHEN** an agent is running eight `node` processes and the section is collapsed
- **THEN** the header names `node` once, not eight times

#### Scenario: More processes than the header names

- **WHEN** an agent is running more distinct processes than the header lists names for
- **THEN** the header names as many as it lists and carries a remainder counting the processes
  those names do not cover

#### Scenario: Expanded section counts

- **WHEN** the user expands the processes section for an agent with three descendants
- **THEN** the header shows three, and does not name them

#### Scenario: Nothing running

- **WHEN** a running agent's section is shown and its last sample found no descendants
- **THEN** the summary says nothing is running, in both the collapsed and expanded states

#### Scenario: A stopped agent does not keep naming processes

- **WHEN** an agent stops while its section holds a sample that listed processes
- **THEN** the summary says nothing is running, rather than naming the processes that sample
  held

#### Scenario: Expanding lists the processes

- **WHEN** the user expands the processes section
- **THEN** one row per descendant is shown, background descendants before foreground ones, each
  longest-running first, and each row shows command, runtime, process identifier, and
  classification

#### Scenario: A long command line stays on one line

- **WHEN** a descendant's command line is wider than the row
- **THEN** the row renders a single line ending in an ellipsis, and the runtime, process
  identifier and actions stay visible

#### Scenario: A long summary does not widen the pane

- **WHEN** the collapsed header's names are wider than the space beside the label
- **THEN** the summary truncates and the pane keeps its width

#### Scenario: The section appears in both views

- **WHEN** the selected agent is shown in the panel view, and again when shown in the terminal
  view
- **THEN** the processes section is present in both

### Requirement: Sampling happens off the render path and only while observed

The system SHALL NOT enumerate processes during a render pass. Samples SHALL be taken on a
recurring interval on a background task, and the rendered rows and header summary SHALL come
from the most recent completed sample.

Sampling for an agent SHALL run while that agent's pane is shown and the agent is running,
whatever the section's disclosure state, and SHALL stop when the agent stops, its pane is no
longer shown, or its window closes.

Disclosure SHALL NOT gate sampling. It previously did, which made the collapsed header's
summary uncomputable: the summary is what tells the user whether expanding is worth it, so it
cannot be the thing that only expanding produces.

At most one agent's pane is shown per window, so the cost SHALL remain one process-table read
per interval per window — not one per agent, and not one per expanded section. A window
showing a takeover view, or whose agent is not running, SHALL sample nothing.

Collapsing the section SHALL NOT discard the last sample. Nothing stale accumulates, because
sampling continues while the section is shut.

#### Scenario: No enumeration during a frame

- **WHEN** the agent's pane re-renders, including on every keystroke
- **THEN** no process enumeration is performed as part of that render, and the rows come from
  the last completed sample

#### Scenario: A collapsed section is still sampled

- **WHEN** a running agent's pane is shown and its processes section has never been expanded
- **THEN** sampling runs for it, and the collapsed header summarizes the sample rather than
  reporting an unknown count

#### Scenario: Collapsing does not stop the sampling or discard the sample

- **WHEN** the user collapses an agent's processes section
- **THEN** sampling continues and the header keeps summarizing the current sample

#### Scenario: One read serves the window

- **WHEN** a window holds six agents and shows one of them
- **THEN** one process-table read is performed per interval, for the agent shown

#### Scenario: Stopping the agent stops the sampling

- **WHEN** a running agent whose processes section is shown is deactivated
- **THEN** sampling for it stops, and the section reports that the agent is not running

#### Scenario: A takeover view stops the sampling

- **WHEN** the dashboard or the pull requests list replaces the agent's pane
- **THEN** sampling stops, because the section is not shown

#### Scenario: Summary before the first sample

- **WHEN** a shown section's first sample has not completed yet
- **THEN** the summary renders as unknown, not as zero and not as nothing running

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
