# Spec Delta

## ADDED Requirements

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
