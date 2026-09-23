# Spec Delta

## MODIFIED Requirements

### Requirement: Command runner

The system SHALL run git commands with an explicit working directory and a
timeout (default 30 seconds). On timeout the process SHALL be terminated and
the call SHALL fail with a timeout error naming the command. A non-zero exit
SHALL fail with an error carrying the command, the stderr (or stdout when
stderr is empty), and the exit code. Output SHALL be returned trimmed of
surrounding whitespace.

The binary the runner spawns SHALL be nameable by the application, once,
before the first command runs, together with the `PATH` the child is given.
Nothing named SHALL mean the bare `git`, resolved by the spawn itself and
inheriting this process's environment, so a caller that configures nothing
behaves as it always did.

The system SHALL NOT itself search for a git binary: locating one among the
standard install locations belongs to the layer that owns that roster, and
this one stays free of the dependency. Naming a git that does not exist SHALL
fail the way a missing git has always failed, at the spawn.

The child's `PATH` exists because git runs helpers of its own - credential
helpers, hooks, `git-lfs`, `ssh` - and looks them up in the environment it is
handed rather than in the one the application was launched with.

#### Scenario: Timeout terminates the process

- **WHEN** a git command runs longer than the timeout
- **THEN** the process is terminated and the result is a timeout error naming
  the command

#### Scenario: Non-zero exit surfaces stderr

- **WHEN** a git command exits non-zero with stderr text
- **THEN** the error carries the command, that stderr, and the exit code

#### Scenario: Nothing configured

- **WHEN** no git has been named and a command runs
- **THEN** the bare `git` is spawned and the child inherits the process's
  environment

#### Scenario: A git named by the application

- **WHEN** the application names an absolute git path and a `PATH`, and a
  command runs afterwards
- **THEN** that binary is spawned, with that `PATH`, without the call site
  naming either

#### Scenario: Named twice

- **WHEN** a second caller names a different git
- **THEN** the first naming stands and the second reports that it did
  nothing, so no two runners in one process disagree about which git they
  mean
