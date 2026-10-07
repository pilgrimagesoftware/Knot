# Spec Delta

## MODIFIED Requirements

### Requirement: The agent's pane presents a processes section

The selected agent's pane SHALL contain a processes section, present for both
the terminal view and the panel view. The section SHALL be collapsible and
SHALL start collapsed.

Expanded, the section SHALL present two labelled groups in a fixed order:
the agent's subagents first, then its descendant processes. The groups SHALL
be labelled so that neither kind of row can be read as the other. A group with
no rows SHALL state its own empty case rather than be omitted silently, except
that the subagents group SHALL be omitted entirely for an agent whose type
cannot report subagents — there is nothing to say about a question that cannot
be asked.

The header SHALL carry a summary of what the agent has running, which SHALL
differ by disclosure state:

- **Collapsed**, it SHALL name what is running rather than count it: the
  distinct subagent kinds first, then the distinct process names, deduplicated,
  each in the order its group already carries. Subagent kinds come first
  because delegated work is the fact the user cannot obtain anywhere else in
  Knot. Where more is running than the header lists names for, it SHALL carry a
  remainder counting the subagents and processes the listed names do not cover.
- **Expanded**, it SHALL give the number of subagents and the number of
  descendants listed below it. The rows are already the detail, so the header
  does not repeat them.

Where nothing is running — no subagents and no descendants, or an agent that is
not running at all — the summary SHALL say so in words. It SHALL NOT be blank,
and it SHALL NOT be the unknown marker. An agent that is not running SHALL read
as nothing running whatever its last sample or last subagent report held, so
the header cannot contradict the body's "not running".

The unknown marker SHALL be shown only while it is true: the section is shown
and its first process sample has not completed. A subagent list is not sampled
and is therefore never unknown.

Expanded, the processes group SHALL list one row per descendant, background
descendants first, and within each group ordered by elapsed runtime,
longest-running first. Each such row SHALL show the descendant's command line
on a single line, truncating with an ellipsis rather than wrapping; its elapsed
runtime; its process identifier; and its background/foreground classification.

Expanded, the subagents group SHALL list one row per subagent, running
subagents first, and within each group ordered by elapsed time, longest-running
first. Each such row SHALL show the subagent's kind; the task it was given, on
a single line, truncating with an ellipsis rather than wrapping; its elapsed
time; and its state. A subagent row SHALL NOT show a process identifier,
because a subagent has none.

#### Scenario: Collapsed section names what is running

- **WHEN** an agent is running `node` and `rg` and its processes section is
  collapsed
- **THEN** the header names both and shows no rows

#### Scenario: Collapsed section names subagents before processes

- **WHEN** an agent is running a `discovery` subagent and a `node` process and
  its section is collapsed
- **THEN** the header names the subagent kind before the process name

#### Scenario: Repeats of one name are folded together

- **WHEN** an agent is running eight `node` processes and the section is
  collapsed
- **THEN** the header names `node` once, not eight times

#### Scenario: Repeats of one subagent kind are folded together

- **WHEN** an agent is running three `code-review` subagents and the section is
  collapsed
- **THEN** the header names `code-review` once, not three times, and carries no
  remainder — the listed name covers all three

#### Scenario: More processes than the header names

- **WHEN** an agent is running more distinct subagent kinds and process names
  than the header lists names for
- **THEN** the header names as many as it lists and carries a remainder
  counting the subagents and processes those names do not cover

#### Scenario: Expanded section counts

- **WHEN** the user expands the processes section for an agent with two
  subagents and three descendants
- **THEN** the header shows two subagents and three processes, and names
  neither

#### Scenario: Nothing running

- **WHEN** a running agent's section is shown, its last sample found no
  descendants and it has dispatched no subagents
- **THEN** the summary says nothing is running, in both the collapsed and
  expanded states

#### Scenario: A stopped agent does not keep naming processes

- **WHEN** an agent stops while its section holds a sample that listed
  processes and records that listed subagents
- **THEN** the summary says nothing is running, rather than naming what those
  held

#### Scenario: Expanding lists the processes

- **WHEN** the user expands the processes section for an agent with both
  subagents and descendant processes
- **THEN** the subagents group is shown first, then the processes group, each
  labelled, with the processes group's rows ordered background before
  foreground and longest-running first within each

#### Scenario: Subagent rows are ordered by state then runtime

- **WHEN** the subagents group holds two running and one finished subagent
- **THEN** the running ones are listed first, longest-running first, and the
  finished one last

#### Scenario: A subagent row carries no process identifier

- **WHEN** a subagent row is rendered
- **THEN** it shows the subagent's kind, task, elapsed time and state, and no
  process identifier

#### Scenario: The subagents group is absent for an unreportable type

- **WHEN** the section is expanded for an agent whose type cannot report
  subagents
- **THEN** the subagents group is not shown at all, and the processes group is
  shown as before

#### Scenario: A long command line stays on one line

- **WHEN** a descendant's command line is wider than the row
- **THEN** the row renders a single line ending in an ellipsis, and the
  runtime, process identifier and actions stay visible

#### Scenario: A long task stays on one line

- **WHEN** a subagent's task is wider than the row
- **THEN** the row renders a single line ending in an ellipsis, and the elapsed
  time, state and actions stay visible

#### Scenario: A long summary does not widen the pane

- **WHEN** the collapsed header's names are wider than the space beside the
  label
- **THEN** the summary truncates and the pane keeps its width

#### Scenario: The section appears in both views

- **WHEN** the selected agent is shown in the panel view, and again when shown
  in the terminal view
- **THEN** the processes section is present in both

### Requirement: Sampling happens off the render path and only while observed

The system SHALL NOT enumerate processes during a render pass. Samples SHALL be
taken on a recurring interval on a background task, and the rendered process
rows SHALL come from the most recent completed sample. The header summary SHALL
come from that sample together with the subagent records already held, neither
of which is read from the operating system during a render.

Sampling for an agent SHALL run while that agent's pane is shown and the agent
is running, whatever the section's disclosure state, and SHALL stop when the
agent stops, its pane is no longer shown, or its window closes.

Subagent records are not sampled: they are built from reports as they arrive,
and are therefore available whether or not the pane is shown. Showing the pane
SHALL NOT be a condition of recording them, because a subagent dispatched while
the user was looking elsewhere is exactly the one worth showing on return.

Disclosure SHALL NOT gate sampling. It previously did, which made the collapsed
header's summary uncomputable: the summary is what tells the user whether
expanding is worth it, so it cannot be the thing that only expanding produces.

At most one agent's pane is shown per window, so the cost SHALL remain one
process-table read per interval per window — not one per agent, and not one per
expanded section. A window showing a takeover view, or whose agent is not
running, SHALL sample nothing.

Collapsing the section SHALL NOT discard the last sample, nor the subagent
records. Nothing stale accumulates, because sampling continues while the
section is shut.

#### Scenario: No enumeration during a frame

- **WHEN** the agent's pane re-renders, including on every keystroke
- **THEN** no process enumeration is performed as part of that render, and the
  rows come from the last completed sample

#### Scenario: A collapsed section is still sampled

- **WHEN** a running agent's pane is shown and its processes section has never
  been expanded
- **THEN** sampling runs for it, and the collapsed header summarizes the sample
  rather than reporting an unknown count

#### Scenario: Collapsing does not stop the sampling or discard the sample

- **WHEN** the user collapses an agent's processes section
- **THEN** sampling continues and the header keeps summarizing the current
  sample

#### Scenario: One read serves the window

- **WHEN** a window holds six agents and shows one of them
- **THEN** one process-table read is performed per interval, for the agent
  shown

#### Scenario: Stopping the agent stops the sampling

- **WHEN** a running agent whose processes section is shown is deactivated
- **THEN** sampling for it stops, and the section reports that the agent is not
  running

#### Scenario: A takeover view stops the sampling

- **WHEN** the dashboard or the pull requests list replaces the agent's pane
- **THEN** sampling stops, because the section is not shown

#### Scenario: A subagent dispatched behind a hidden pane is recorded

- **WHEN** an agent dispatches a subagent while another agent's pane is shown
- **THEN** the subagent is recorded, and appears when that agent's pane is next
  shown

#### Scenario: Summary before the first sample

- **WHEN** a shown section's first process sample has not completed yet
- **THEN** the summary renders as unknown, not as zero and not as nothing
  running

### Requirement: A listed process can be terminated

Each process row SHALL offer a terminate action. Activating it SHALL first ask
the user to confirm, naming the process's command line, and SHALL do nothing if
the user cancels.

On confirmation the system SHALL send the process a termination signal, wait a
bounded grace period for it to exit, and, if it is still alive when the grace
period elapses, send a kill signal. Terminating a process SHALL target only
that process, never its siblings, never its parent, and never the agent's
session root.

If the process has already exited by the time the action runs, the system SHALL
treat this as success and report nothing as an error. If the signal is refused
— for example because the process is not owned by the user running Knot — the
system SHALL surface a localized failure and leave the list unchanged.

A subagent row SHALL NOT offer a terminate action. A subagent has no process
identifier to signal, and the only way to stop one is to interrupt the parent
agent, which is a different action with a different consequence; offering
`Terminate` on a row where it would mean that is worse than not offering it.

#### Scenario: Confirmed termination ends the process

- **WHEN** the user confirms terminating a listed process that ignores nothing
- **THEN** the process is sent a termination signal and is absent from the next
  sample

#### Scenario: A process that ignores termination is killed

- **WHEN** a process does not exit within the grace period after the
  termination signal
- **THEN** it is sent a kill signal

#### Scenario: Cancelling changes nothing

- **WHEN** the user opens the terminate confirmation and cancels
- **THEN** no signal is sent and the process keeps running

#### Scenario: Terminating one process leaves the others

- **WHEN** the user terminates one of an agent's several descendants
- **THEN** only that process is signalled; the agent's session root and its
  other descendants keep running

#### Scenario: Already-exited process

- **WHEN** the user confirms terminating a process that has exited since the
  last sample
- **THEN** no error is surfaced and the row disappears on the next sample

#### Scenario: A subagent row offers no termination

- **WHEN** a running subagent's row is rendered
- **THEN** it offers no terminate action, and no action on it signals the
  parent agent

### Requirement: A listed process can be identified outside Knot

Each process row SHALL offer actions that carry the process's identity out of
Knot: copying its process identifier, copying its full command line, and
opening the platform's process viewer.

Each subagent row SHALL offer an action copying the subagent's full task.
It SHALL offer no process-identifier action and no process-viewer action,
because a subagent has neither.

Copying SHALL place the exact value on the system clipboard, unabbreviated —
the full command line or the full task, not the truncated text the row
displays.

On macOS the process-viewer action SHALL open Activity Monitor. The platform
offers no supported way to select a given process inside Activity Monitor from
outside it, so the action SHALL open the application only; the row's
copy-identifier action is what carries the process identifier across. On a
platform with no known process viewer the action SHALL NOT be offered.

#### Scenario: Copying the command copies it in full

- **WHEN** the user copies the command of a row whose display text is truncated
- **THEN** the clipboard holds the complete command line, with no ellipsis

#### Scenario: Copying the identifier

- **WHEN** the user copies a row's process identifier
- **THEN** the clipboard holds that identifier

#### Scenario: Copying a subagent's task copies it in full

- **WHEN** the user copies the task of a subagent row whose display text is
  truncated
- **THEN** the clipboard holds the complete task, with no ellipsis

#### Scenario: Opening the process viewer on macOS

- **WHEN** the user activates the process-viewer action on macOS
- **THEN** Activity Monitor is opened

### Requirement: The section states why it is empty

When the processes section is expanded, each group with no rows to show SHALL
say which case it is in rather than render blank.

For the processes group: the agent is not running, the agent is running and has
spawned nothing, or the last sample failed.

For the subagents group: the agent is not running, or the agent is running and
has dispatched none. An agent whose type cannot report subagents is not an
empty case — that group is not shown at all, so the section never states that
such an agent dispatched nothing.

A failed sample SHALL NOT clear a previously shown process list; the group
SHALL keep showing the last successful sample alongside the failure, and SHALL
recover silently when a later sample succeeds. A failed sample SHALL NOT affect
the subagents group, which does not come from sampling.

#### Scenario: Agent not running

- **WHEN** the processes section is expanded for an agent that is not running
- **THEN** each shown group states that the agent is not running

#### Scenario: Running with nothing spawned

- **WHEN** the processes section is expanded for a running agent whose session
  root has no descendants
- **THEN** the processes group states that the agent has spawned nothing

#### Scenario: Running with nothing dispatched

- **WHEN** the processes section is expanded for a running agent that can
  report subagents and has dispatched none
- **THEN** the subagents group states that the agent has dispatched none

#### Scenario: An unreportable type states nothing

- **WHEN** the processes section is expanded for a running agent whose type
  cannot report subagents
- **THEN** no subagents group is shown, and nothing states that it dispatched
  none

#### Scenario: A failed sample keeps the last list

- **WHEN** a sample fails after an earlier sample succeeded
- **THEN** the rows from the last successful sample stay shown, with the
  failure reported alongside them

#### Scenario: A failed sample leaves the subagents alone

- **WHEN** a process sample fails while subagents are listed
- **THEN** the subagent rows are unaffected and carry no failure notice

#### Scenario: Recovery is silent

- **WHEN** a sample succeeds after a failed one
- **THEN** the failure notice is cleared without further user action

### Requirement: All process-section text is localized

Every user-facing string this capability introduces — the section label, the
group labels, the counts, the column and classification labels, the subagent
state labels, each row action, the terminate confirmation, the empty states,
and the failure notices — SHALL be resolved through the application's
localization lookup.

A process's own command line and identifier are data and SHALL be shown
verbatim. A subagent's kind and task are likewise the agent's own data and
SHALL be shown verbatim; the kind is an open vocabulary drawn from the user's
configuration and SHALL NOT be looked up for translation.

#### Scenario: Section text resolves through localization

- **WHEN** the processes section renders its label, group labels, counts,
  actions, subagent states and empty states
- **THEN** each of those strings is resolved by localization key

#### Scenario: Process data is not localized

- **WHEN** a row renders a process's command line and identifier
- **THEN** they are shown exactly as the operating system reports them

#### Scenario: Subagent data is not localized

- **WHEN** a subagent row renders its kind and task
- **THEN** they are shown exactly as the agent reported them
