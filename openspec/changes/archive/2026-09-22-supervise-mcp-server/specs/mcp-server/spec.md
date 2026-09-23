# Spec Delta

## ADDED Requirements

### Requirement: Observable server lifecycle state

The MCP server SHALL expose a lifecycle state that the rest of the application can observe
and that changes as the server starts, runs, fails, and recovers. The state SHALL be one of:

- **disabled** — configuration has the server turned off; nothing is bound and nothing is
  supervised.
- **starting** — a bind is in progress.
- **running** — bound and serving, carrying the bound address.
- **retrying** — the last attempt failed, carrying the attempt count, the delay before the next
  attempt, and the error that caused it.
- **stopped** — the application asked the server to stop.

Every transition SHALL be observable by a watcher that subscribed before the transition
occurred, and a watcher that subscribes late SHALL immediately see the current state rather
than wait for the next change.

#### Scenario: State reaches running after a successful bind

- **WHEN** the server binds its configured port successfully
- **THEN** its state becomes running and carries the bound address

#### Scenario: A late watcher sees the current state

- **WHEN** a watcher subscribes after the server has already reached running
- **THEN** it immediately observes running, without waiting for another transition

#### Scenario: Disabled by configuration

- **WHEN** the MCP server is disabled by configuration
- **THEN** its state is disabled, no port is bound, and no supervision runs

### Requirement: An unexpectedly ended serve task is restarted

While the server's state is running, the system SHALL watch the task serving requests. If that
task ends for any reason other than the application asking the server to stop — it returns, it
panics, its listener is closed underneath it — the system SHALL treat it as a failure, tear
down any remaining server resources, and start the server again on the configured port.

A restart SHALL reuse the same tool catalog, hook handler, and agent snapshot source the
server was constructed with; it SHALL NOT require agents to register again.

#### Scenario: The serve task returns

- **WHEN** the serve task ends on its own while the state is running
- **THEN** the state leaves running and the server is started again on the configured port

#### Scenario: The serve task panics

- **WHEN** the serve task ends by panicking
- **THEN** it is treated as a failure like any other, and the server is started again

#### Scenario: A restart preserves the served surface

- **WHEN** the server has restarted after a failure
- **THEN** the same tools, hook handling, and status data are served as before the failure, and
  agents that were registered remain registered

### Requirement: A server that stops answering is restarted

While the state is running, the system SHALL probe the server's own health endpoint on a
recurring interval. A probe SHALL be considered failed when it does not return a success
response within a bounded timeout. After a configured number of consecutive failed probes, the
system SHALL treat the server as failed and restart it exactly as it does an ended serve task.

A single failed probe SHALL NOT trigger a restart, and one successful probe SHALL reset the
consecutive-failure count to zero.

#### Scenario: Repeated probe failures restart the server

- **WHEN** the health probe fails for the configured number of consecutive attempts
- **THEN** the server is torn down and started again

#### Scenario: One failed probe is tolerated

- **WHEN** a single health probe fails and the next succeeds
- **THEN** no restart occurs and the consecutive-failure count returns to zero

#### Scenario: Probing stops outside the running state

- **WHEN** the server's state is starting, retrying, stopped, or disabled
- **THEN** no health probes are issued

### Requirement: Start attempts retry on the configured port with backoff

A failed attempt to bind the server SHALL NOT end supervision. The system SHALL retry, waiting
between attempts for a delay that grows exponentially from an initial value up to a maximum,
and SHALL keep retrying for as long as the server is enabled and has not been asked to stop.

Every attempt SHALL target the port the configuration names. The system SHALL NOT bind a
different or ephemeral port in response to a failure: the URL agents are launched with is built
from the configured port, so a server that recovered elsewhere would be unreachable to every
agent already running.

Between attempts the state SHALL be retrying, carrying the attempt count, the delay before the
next attempt, and the error from the last attempt.

#### Scenario: A busy port is retried

- **WHEN** the configured port is held by another process at start
- **THEN** the state becomes retrying and further attempts are made on that same port

#### Scenario: Recovery when the port is released

- **WHEN** the process holding the configured port exits between two attempts
- **THEN** the next attempt binds successfully and the state becomes running

#### Scenario: Backoff grows and is capped

- **WHEN** attempts continue to fail
- **THEN** the delay between them grows from the initial delay and stops growing at the maximum
  delay, rather than growing without bound

#### Scenario: The port never changes

- **WHEN** any number of attempts fail
- **THEN** no attempt binds a port other than the configured one

### Requirement: An intentional stop is never restarted

When the application asks the server to stop — shutdown, or the server being disabled by
configuration — the system SHALL stop supervision first, so the serve task ending is recognized
as the expected consequence of that request and not as a failure. After an intentional stop the
state SHALL be stopped or disabled, no restart SHALL occur, no health probe SHALL be issued,
and the port SHALL be released.

#### Scenario: Shutdown does not trigger a restart

- **WHEN** the application asks the MCP server to stop
- **THEN** the state becomes stopped, the server does not start again, and the port is released

#### Scenario: Disabling the server does not trigger a restart

- **WHEN** the MCP server is disabled by configuration while running
- **THEN** supervision ends, the state becomes disabled, and no restart occurs
