# Spec Delta

## ADDED Requirements

### Requirement: The tool is found where it is installed

The tool Knot reads pull request state through SHALL be located on a search
path made of the process's own `PATH` followed by the standard non-sandbox
install locations, with those locations appended only where the process
`PATH` does not already name them, and `~`-prefixed locations resolved
against the `HOME` environment variable. This is the same merged search path
`agent-launch-command` specifies for ACP adapters, and SHALL be the same list
of locations, so a tool found by one part of the application is not reported
missing by another.

The tool SHALL be spawned by the absolute path it was located at, so that
"not installed" is a conclusion drawn from a search of named directories
rather than an inference from a spawn error. Setting the subprocess's `PATH`
alone happens to suffice on the current Rust standard library, which drops to
`fork`/`exec` so the supplied environment is the one searched; that is an
implementation detail of `std` rather than a guarantee it documents, and this
requirement does not rest on it.

The subprocess SHALL also be given the merged search path as its `PATH`: it
runs `git` and credential helpers of its own, which are subject to the same
launchd `PATH` that motivates this requirement.

Only when no directory on the merged path holds the tool SHALL it be reported
as not installed.

#### Scenario: Installed outside the launchd PATH

- **WHEN** the app was launched from Finder (so its `PATH` is
  `/usr/bin:/bin:/usr/sbin:/sbin`) and the tool is installed in
  `/opt/homebrew/bin` or another standard install location
- **THEN** state is fetched for the recorded pull requests, and no message
  says the tool is not installed

#### Scenario: Genuinely not installed

- **WHEN** no directory on the merged search path holds the tool
- **THEN** the view reports that the tool is not installed, exactly as it does
  today

#### Scenario: The tool's own subprocesses

- **WHEN** the tool runs `git` or a credential helper while answering a
  request
- **THEN** it does so under the merged search path rather than the app's
  inherited one
