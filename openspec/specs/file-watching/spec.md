# file-watching Specification

## Purpose
Defines filesystem monitoring used to auto-refresh git status and repository
discovery: a recursive watch on a directory, a debounce before firing, a
pause/resume control used around the app's own git writes, and a relevance
filter that suppresses irrelevant change bursts.

## Requirements

### Requirement: Debounced directory watch

The system SHALL watch a directory tree for filesystem changes and invoke a
single callback after changes settle, debouncing bursts (git status watch
about 1 second; generic file watch about 0.3 second). Starting an
already-running watch SHALL be a no-op; stopping SHALL cancel any pending
callback.

#### Scenario: Burst collapses to one callback

- **WHEN** twenty files change within the debounce window
- **THEN** the callback fires once after the window

### Requirement: Pause and resume

The system SHALL support pausing and resuming a watch so the app can suppress
self-inflicted events during its own git operations (stage, commit, discard).
Events delivered while paused SHALL NOT invoke the callback; a short settle
delay SHALL elapse after resume before events are honored again.

#### Scenario: Own commit does not self-trigger

- **WHEN** the watch is paused, a commit runs, and the watch resumes
- **THEN** the commit's filesystem changes do not fire the refresh callback

### Requirement: Relevance filter for discovery

For the source-folder watch, a change SHALL be considered relevant only when
its path is within the watched base and it touches the base directly or a
first- or second-level entry (a repository folder or its `.git`); deeper
working-tree noise SHALL be ignored.

#### Scenario: Deep edit is ignored

- **WHEN** a file changes several levels inside a repository's working tree
- **THEN** the discovery rescan is not triggered

#### Scenario: New repo folder is relevant

- **WHEN** a new folder appears directly under the source folder
- **THEN** the rescan is triggered

### Requirement: Relevance filter for a git working tree

The git-status watch over a working tree SHALL treat a changed path as relevant
only when it could change what `git status` reports, so that routine git and
editor churn does not drive a refresh.

Within the tree's `.git` directory, only the index, `HEAD`, and anything under
`refs/` SHALL be relevant. Everything else under `.git` — objects, logs,
`COMMIT_EDITMSG`, hooks, configuration — SHALL be ignored: git rewrites those
constantly during its own operations, and none of them changes the working
tree's status.

Outside `.git`, a path whose final component begins with a dot SHALL be ignored,
except `.gitignore`, which is relevant because editing it changes which files
are untracked. All other paths SHALL be relevant.

A batch of changes SHALL drive at most one refresh, whatever its size: relevance
is a question about the batch, not a count. A batch containing no relevant path
SHALL NOT extend a pending debounce.

This filter is the git-panel counterpart to the source-folder filter in
"Relevance filter for discovery"; the two watches answer different questions and
do not share a rule.

#### Scenario: Staging is relevant

- **WHEN** the `.git/index` of the watched tree changes
- **THEN** the change is relevant and a refresh follows once changes settle

#### Scenario: A commit or branch switch is relevant

- **WHEN** `.git/HEAD` or a file under `.git/refs/` changes
- **THEN** the change is relevant

#### Scenario: Git's internal churn is ignored

- **WHEN** a file under `.git/objects`, `.git/logs`, or `.git/COMMIT_EDITMSG`
  changes and nothing else does
- **THEN** no refresh follows

#### Scenario: A tracked file edit is relevant

- **WHEN** a file in the working tree outside `.git` is edited
- **THEN** the change is relevant

#### Scenario: Dotfiles are ignored except .gitignore

- **WHEN** a `.DS_Store` file changes and nothing else does
- **THEN** no refresh follows
- **WHEN** the tree's `.gitignore` changes
- **THEN** the change is relevant

#### Scenario: A burst collapses to one refresh

- **WHEN** fifty files in the working tree change at once
- **THEN** exactly one refresh follows after the changes settle
