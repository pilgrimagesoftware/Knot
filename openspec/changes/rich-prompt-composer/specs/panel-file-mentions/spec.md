# Spec Delta

## Purpose

Lets the user name a file from the agent's working folder by typing `@` in
the panel composer, completing a relative path into the prompt rather than
opening a separate finder window.

## ADDED Requirements

### Requirement: `@` opens a file lookup

When the user types `@` at a word boundary in the panel composer, the system
SHALL show the lookup popup listing files from the focused agent's working
folder. The list SHALL narrow to the files matching the text typed after the
`@`.

The Swift reference reaches the same file list through a modal finder sheet
opened by a keyboard shortcut. The port SHALL reach it inline from the
composer instead: the user is already typing the prompt the path belongs in,
and a modal takes the prompt off screen to do it.

#### Scenario: `@` opens the lookup

- **WHEN** the user types `@` after a space or at the start of the buffer
- **THEN** the lookup popup appears above the composer listing files from the
  agent's working folder

#### Scenario: Typing narrows the list

- **WHEN** the user types characters after the `@`
- **THEN** the list shows only files matching the typed text

#### Scenario: `@` mid-word is not a trigger

- **WHEN** the user types `@` immediately after a non-space character, as in
  an email address
- **THEN** no lookup opens

### Requirement: Matching is fuzzy and ranked

The lookup SHALL match the typed text against each file's path as a
subsequence rather than a contiguous substring, so `kgs` reaches
`knot-git/src`. Matches SHALL be ordered best-first, and the ordering SHALL
prefer matches on the file's name over matches on its parent directories, and
consecutive matched characters over scattered ones.

The matched characters SHALL be marked in each listed row, so the user can
see why a row is a match.

This is the Swift reference's behaviour, moved from the finder sheet to the
composer unchanged in substance.

#### Scenario: A subsequence matches

- **WHEN** the user types `@kgs`
- **THEN** `crates/knot-git/src/lib.rs` is among the matches

#### Scenario: Name matches outrank directory matches

- **WHEN** the typed text matches one file's name and another file's parent
  directory equally well
- **THEN** the file matched on its name is listed first

#### Scenario: Matched characters are marked

- **WHEN** the lookup lists a match
- **THEN** the characters that matched the typed text are distinguished
  within the row's path

### Requirement: The listing is the agent's tracked working tree

The files offered SHALL be those of the focused agent's own working folder —
its worktree when it has one, not the primary checkout and not another
agent's.

When that folder is a git repository, the listing SHALL be the repository's
tracked and untracked-but-not-ignored files, so a build directory the
repository ignores is not offered. When it is not a repository, the listing
SHALL be a filesystem walk that skips version-control metadata and the
conventional dependency and build directories.

#### Scenario: Ignored files are not offered

- **WHEN** the agent's folder is a git repository whose `.gitignore` excludes
  `target/`
- **THEN** no path under `target/` appears in the lookup

#### Scenario: A new untracked file is offered

- **WHEN** the user creates a file the repository does not ignore and has not
  yet staged
- **THEN** it appears in the lookup

#### Scenario: Each agent sees its own worktree

- **WHEN** two agents in a workspace occupy different worktrees of one
  repository
- **THEN** each agent's lookup lists that agent's own worktree

### Requirement: Listing never blocks typing

Enumerating the folder SHALL happen off the render path. The composer SHALL
stay responsive to typing while enumeration runs, and the popup SHALL say it
is still gathering files rather than showing an empty list that is merely not
ready.

The listing SHALL be gathered once per agent and reused, not rebuilt per
keystroke, and SHALL be refreshed when the folder's contents change rather
than on a timer.

A folder too large to enumerate within its cap SHALL be reported as such in
the popup, with matching still offered over the files gathered so far.

#### Scenario: Typing during enumeration

- **WHEN** the user types `@` in an agent whose folder has not been
  enumerated yet
- **THEN** the popup states that files are still being gathered and the
  composer continues to accept keystrokes

#### Scenario: A file appears after it is created

- **WHEN** a file is created in the agent's folder while the agent is open
- **THEN** a subsequent `@` lookup offers it without the user reopening the
  agent

#### Scenario: An oversized folder is reported

- **WHEN** the agent's folder holds more files than the enumeration cap
- **THEN** the popup says the folder is too large to list completely and
  still matches against what was gathered

### Requirement: Inserting a mention completes a path

Inserting an entry SHALL replace the partially typed `@` token with `@`
followed by the file's path relative to the agent's working folder, and
SHALL leave the rest of the buffer untouched. Insertion SHALL NOT read,
attach or upload the file; the completed text is sent to the agent like any
other prompt, and what the agent does with the path is the agent's business.

A path containing a space SHALL be inserted in a form that keeps it one
token.

#### Scenario: The token is replaced in place

- **WHEN** the user has typed `look at @knotg` and inserts
  `crates/knot-git/src/lib.rs`
- **THEN** the buffer reads `look at @crates/knot-git/src/lib.rs` and the
  text before the token is unchanged

#### Scenario: Inserting does not attach the file

- **WHEN** the user inserts a mention for an image file
- **THEN** no attachment appears in the strip above the input and no file is
  read

#### Scenario: A path with a space stays one token

- **WHEN** the user inserts a file whose path contains a space
- **THEN** the inserted text keeps the path as a single token

### Requirement: One popup, one active lookup

The `@` lookup and the slash lookup SHALL share a single popup and SHALL
never both be open. The token under the caret SHALL decide which lookup is
active; when neither trigger is under the caret, the popup SHALL be closed.

The `@` lookup SHALL take the same keys as the slash lookup: Up and Down move
the selection, Enter or Tab inserts, and Esc dismisses without changing the
buffer.

#### Scenario: Moving between the two tokens

- **WHEN** the buffer holds both a slash token and an `@` token and the caret
  moves from one to the other
- **THEN** the popup switches to the lookup for the token under the caret,
  and only one list is shown

#### Scenario: Esc dismisses the mention lookup

- **WHEN** the user presses Esc while the `@` lookup is open
- **THEN** the popup closes and the buffer is unchanged

#### Scenario: Leaving the token closes the popup

- **WHEN** the user edits away the `@` or moves focus out of the composer
- **THEN** the popup closes
