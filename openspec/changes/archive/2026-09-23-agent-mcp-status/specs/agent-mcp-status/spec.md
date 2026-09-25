# Spec Delta

## Purpose

Makes the MCP servers behind a Panel-mode agent visible and actionable from
that agent's own pane: which servers its configuration defines, which of them
are connected and which need authentication or have failed, and a way to reach
the agent's own remedy without leaving Knot. The Swift reference app has no
equivalent — it showed only Knot's own MCP server and its install command —
so this is new behavior in the Rust port rather than a transcription.

## ADDED Requirements

### Requirement: A Panel-mode agent's pane exposes an MCP servers section

An agent shown in Panel mode SHALL expose an MCP servers section in its own
pane, collapsible, independent of the processes section and of any other
disclosure state. An agent shown in Terminal mode SHALL NOT expose the
section: the agent's own MCP command is reachable by typing it into that
agent's terminal.

#### Scenario: Panel-mode agent has the section

- **WHEN** an agent whose view mode is Panel is shown
- **THEN** its pane includes an MCP servers section

#### Scenario: Terminal-mode agent does not

- **WHEN** an agent whose view mode is Terminal is shown
- **THEN** its pane includes no MCP servers section

#### Scenario: Switching view mode

- **WHEN** a Panel-mode agent is switched to Terminal mode
- **THEN** the MCP servers section is no longer shown, and any probe in
  flight for that agent is abandoned rather than reported

### Requirement: The section lists both Knot's server and the agent's own

The expanded section SHALL list, as rows of one list, Knot's own MCP server
and every MCP server the agent's own configuration defines. Each row SHALL
carry the server's name, its transport, and its state. Knot's own server's
state SHALL come from Knot's supervision of that server, not from probing the
agent; the agent's servers' states SHALL come from the probe.

Knot's own server SHALL appear in the list whenever it is configured to run,
including when it is disabled or has failed to start, and SHALL be
identifiable as Knot's own rather than one of the agent's.

#### Scenario: Both sources appear

- **WHEN** the section is expanded for an agent whose configuration defines
  two MCP servers and Knot's MCP server is running
- **THEN** the list holds three rows: Knot's server reported as running, and
  the agent's two servers with the states the probe found

#### Scenario: Knot's server is disabled

- **WHEN** the MCP server is disabled in settings and the section is expanded
- **THEN** Knot's own row is present and reports the disabled state, rather
  than being omitted

#### Scenario: Knot's state does not wait on the probe

- **WHEN** the probe has not yet completed
- **THEN** Knot's own row already shows its current state

### Requirement: Knot's server appears once even when the agent also configures it

A user may register Knot's MCP server in an agent's own configuration as well
as receiving it through the session Knot opens. When the probe reports a
server that is Knot's own, the system SHALL show one row for it, not two, and
that row SHALL carry the state Knot's own supervision reports. The row SHALL
say that the agent configures it independently, because that copy outlives
the session Knot controls and is the user's to remove.

Identity SHALL be decided by what the server points at, not by its name
alone: a differently-named entry addressing Knot's own MCP endpoint is the
same server, and a same-named entry addressing something else is not.

#### Scenario: Registered in both places

- **WHEN** the agent's configuration defines a server addressing Knot's own
  MCP endpoint
- **THEN** the list holds a single row for Knot's server, showing the state
  Knot reports, and noting that the agent configures it independently

#### Scenario: A different server under the same name

- **WHEN** the agent's configuration defines a server named like Knot's but
  addressing a different endpoint
- **THEN** it is listed as one of the agent's own servers, separately from
  Knot's row

### Requirement: Server state is a closed vocabulary

A server's state SHALL be one of a fixed set — connected, needs
authentication, pending approval, disabled, failed, unknown — with no
open-ended fallback value. A probe result the system cannot classify SHALL
map to unknown rather than to connected or to a passed-through string.

Pending approval and disabled are states the agent's own tooling reports for
a server it is deliberately not connecting to. They SHALL be distinct from
failed: nothing is broken, and the remedy is the user's decision rather than
a repair.

#### Scenario: An unclassifiable result is unknown

- **WHEN** the probe reports a server in terms the system does not recognize
- **THEN** that server's state is unknown

#### Scenario: Needing authentication is distinct from failing

- **WHEN** the probe reports one server as requiring authentication and
  another as failing to connect
- **THEN** the two rows show distinct states, and the one requiring
  authentication is not described as failed

#### Scenario: An unapproved server is not a failed one

- **WHEN** the probe reports a server as awaiting the user's approval, and
  another as disabled for this project
- **THEN** those rows report pending approval and disabled respectively, and
  neither is reported as failed

### Requirement: The collapsed section names what needs attention

Collapsed, the section's header SHALL say how many of the agent's servers
need attention — that is, are in the needs-authentication or failed state —
and SHALL name them, up to a bounded number, with a remainder counting those
the names do not cover. When nothing needs attention, the header SHALL say
so in terms of the servers it knows about rather than showing a bare count or
an empty label. Expanded, the header SHALL count the rows listed below it.

#### Scenario: Something needs attention

- **WHEN** the section is collapsed and two of the agent's four servers need
  authentication
- **THEN** the header names those two servers as needing attention

#### Scenario: Nothing needs attention

- **WHEN** the section is collapsed and every known server is connected
- **THEN** the header says every server is connected, and names none

#### Scenario: Expanded counts

- **WHEN** the section is expanded
- **THEN** the header counts the rows shown

### Requirement: Not knowing is distinguished from knowing there are none

For an agent whose type the system has no way to interrogate, the section
SHALL report that it cannot determine the agent's MCP servers, and SHALL NOT
report that the agent has none. An empty list SHALL be shown only when a
probe completed and found no servers configured.

#### Scenario: An agent type with no MCP command

- **WHEN** the section is shown for an agent of a type the system has no MCP
  list command for
- **THEN** the section reports that it cannot determine that agent's MCP
  servers, and still shows Knot's own server row

#### Scenario: An agent with no servers configured

- **WHEN** a probe completes and the agent's configuration defines no MCP
  servers
- **THEN** the section reports that the agent has no MCP servers configured

### Requirement: A failed probe is reported rather than shown as emptiness

A probe that cannot run or does not complete — the agent type's command is
not on `PATH`, the command exits with an error, or it exceeds a bounded
timeout — SHALL leave the section reporting that state, naming what failed.
It SHALL NOT be reported as an agent with no servers, and SHALL NOT leave the
section reporting that a probe is still in progress.

#### Scenario: The command is missing

- **WHEN** the agent type's MCP list command is not found on `PATH`
- **THEN** the section reports that the command could not be run, naming it

#### Scenario: The probe times out

- **WHEN** a probe exceeds its timeout
- **THEN** the probe is abandoned, the section reports the timeout, and the
  section does not remain in the in-progress state

#### Scenario: A failed probe does not discard what was known

- **WHEN** a probe fails after an earlier probe succeeded
- **THEN** the rows from the earlier probe remain shown, marked with when
  they were taken, alongside the report of the failure

### Requirement: Probing is bounded and runs off the render path

A probe for an agent SHALL run only while that agent is running and its pane
is shown, and SHALL run on completion of none of the render path's work: no
probe SHALL be started, awaited or read synchronously while a frame is being
produced. A probe's result SHALL reach a rendered frame — a result that
lands between frames SHALL cause a repaint rather than waiting for an
unrelated one.

Probing SHALL NOT run on a fixed fast interval. A probe SHALL be started
when the section first becomes visible for a running agent, when the user
invokes the section's refresh action, and after a delegated action's terminal
exits. At most one probe per agent SHALL be in flight at a time; a request
made while one is in flight SHALL be ignored rather than queued.

#### Scenario: A hidden pane does not probe

- **WHEN** an agent's pane is not shown
- **THEN** no probe runs for that agent

#### Scenario: A stopped agent does not probe

- **WHEN** an agent is not running
- **THEN** no probe runs for it, and the section reports that its state is
  not known for a stopped agent

#### Scenario: Refresh re-probes

- **WHEN** the user invokes the section's refresh action
- **THEN** a probe runs and the section shows its result

#### Scenario: Concurrent refreshes collapse

- **WHEN** the user invokes refresh while a probe for that agent is already
  in flight
- **THEN** no second probe is started and the in-flight one's result is shown

#### Scenario: A result lands on a frame

- **WHEN** a probe completes while no other repaint is pending
- **THEN** the section redraws with the new rows

### Requirement: The section says when its rows were taken

The section SHALL show, alongside the agent's rows, when the probe that
produced them ran. It SHALL NOT present those rows as the running agent's
live session state.

#### Scenario: Rows carry their age

- **WHEN** the section shows rows from a probe that completed earlier
- **THEN** the section shows when that probe ran

### Requirement: A row identifies its server without spilling its command

A row SHALL identify its server by name and transport. Where the server is
reached by a command, the row SHALL NOT render that command in full — a
stdio server's command line is unbounded and routinely runs to thousands of
characters. The row SHALL show a bounded identifier and SHALL make the full
value reachable on demand.

#### Scenario: A long stdio command is not rendered in the row

- **WHEN** a server is reached by a command line longer than the row can
  hold
- **THEN** the row shows a bounded identifier for it, and the row's height is
  unaffected by the command's length

#### Scenario: The full value stays reachable

- **WHEN** the user asks for a row's full target
- **THEN** the system makes the complete command or URL available

### Requirement: A server needing attention delegates to the agent's own flow

A row for one of the agent's servers in the needs-authentication, failed,
pending-approval or disabled state SHALL offer an action that hands the user
that agent type's own MCP flow. The system SHALL open a terminal in the
agent's working directory and enter that agent type's MCP command into it,
leaving the user in that flow.

Where the agent type offers a command that addresses one named server
directly, the action SHALL use it, naming the server on the row. Where it
offers only an interactive flow, the action SHALL enter whatever reaches that
flow for the type. Either way the user completes the flow themselves: the
system SHALL NOT perform authentication, SHALL NOT store credentials, and
SHALL NOT disturb the agent's ACP session.

When that terminal exits, the system SHALL probe again so the section
reflects whatever the user did.

A row in the connected or unknown state SHALL NOT offer the action, and
neither SHALL a row for an agent of a type with no MCP command.

#### Scenario: Delegating for a server needing authentication

- **WHEN** the user invokes the action on a row reporting that the server
  needs authentication
- **THEN** a terminal opens in the agent's working directory with that agent
  type's MCP management command entered, and the agent's ACP session is
  unaffected

#### Scenario: A type with a per-server command uses it

- **WHEN** the user invokes the action on a row of an agent whose type offers
  a command addressing one named server
- **THEN** the terminal receives that command with the row's server named,
  rather than an interactive flow the user must navigate

#### Scenario: The section catches up afterwards

- **WHEN** the terminal opened by the action exits
- **THEN** a probe runs and the section shows its result

#### Scenario: A connected server offers no action

- **WHEN** a row reports its server as connected
- **THEN** that row offers no delegated action

### Requirement: Knot's own server row offers no delegated action

The row for Knot's own MCP server SHALL offer no delegated action. Its
lifecycle is Knot's own: it is supervised and restarted by Knot, and its
configuration is the settings window's MCP tab.

#### Scenario: Knot's row has no action

- **WHEN** Knot's own MCP server is in any state, including failed
- **THEN** its row offers no delegated action

### Requirement: The section never writes the agent's MCP configuration

The system SHALL NOT create, modify or delete any MCP server entry in any
agent's configuration. Every command it runs on the agent's behalf SHALL be
one that reads state; every change to that configuration SHALL happen in the
delegated flow, under the user's own hands.

#### Scenario: Probing does not mutate

- **WHEN** a probe runs for an agent
- **THEN** that agent's MCP configuration is unchanged
