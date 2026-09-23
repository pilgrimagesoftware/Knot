# Spec Delta

## MODIFIED Requirements

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
