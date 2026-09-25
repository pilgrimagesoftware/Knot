# git-panel-ui Specification

## Purpose
Defines the git panel: the sliding overlay over an agent's folder that shows its
working tree grouped by staged state, renders the unified diff for a selected
file, stages, unstages and discards individual paths, and commits — the review
loop that lets someone read an agent's work without leaving Knot.

## Requirements

### Requirement: Opening and dismissing the panel

The git panel SHALL be scoped to one agent's folder and SHALL appear beside that
agent's content within the same window, rather than opening a separate window or
replacing what the window was showing. It SHALL provide an explicit close
control that returns the window to what it was showing.

A panel opened for a folder that is not a git working tree SHALL say so rather
than presenting an empty working tree, because an empty tree and a
non-repository are different answers.

#### Scenario: Panel opens beside the agent's content

- **WHEN** the git panel is opened for an agent
- **THEN** it appears beside that agent's content, scoped to that agent's folder
- **AND** no separate window is created
- **AND** the agent's content remains visible

#### Scenario: Close returns to the previous content

- **WHEN** the panel's close control is activated
- **THEN** the panel is dismissed and the window shows what it showed before

#### Scenario: Folder is not a repository

- **WHEN** the panel is opened for a folder that is not a git working tree
- **THEN** it reports that the folder is not a repository
- **AND** it does not present a clean working tree

### Requirement: Panel width and resizing

The panel SHALL open at a default width and SHALL be resizable by dragging a
handle on its leading edge, clamped to a minimum of 350 and a maximum of 800
points. The default width SHALL be 500 points.

Width is not persisted: a newly opened panel SHALL start at the default width
again. This matches the Swift reference, where panel width is view state rather
than a stored setting.

#### Scenario: Default width on open

- **WHEN** the panel is opened
- **THEN** it is 500 points wide

#### Scenario: Drag resizes within bounds

- **WHEN** the user drags the resize handle
- **THEN** the panel width follows the drag
- **AND** it never becomes narrower than 350 or wider than 800 points

#### Scenario: Width is not remembered

- **WHEN** the panel is resized, closed, and opened again
- **THEN** it is 500 points wide again

### Requirement: Panel content states

The panel SHALL show exactly one of four content states, chosen in this order:
loading, error, clean working tree, and working tree with changes.

The loading state SHALL appear only while no status has ever been loaded for
this panel. A refresh of an already-loaded panel SHALL leave the current content
on screen until the new status arrives, so a routine refresh does not blank the
panel.

**Divergence from Swift**: the Swift reference cannot reach an error state from
a failed `git status` — its repository layer swallows the failure and returns an
empty status, so a broken repository renders as "Working tree clean". Here a
failed status is an error and SHALL be reported as one.

#### Scenario: First load shows loading

- **WHEN** the panel has been opened and no status has arrived yet
- **THEN** it shows a loading state

#### Scenario: Refresh does not blank the panel

- **WHEN** a status has already been shown and a refresh is in flight
- **THEN** the previously loaded content remains on screen
- **AND** the loading state is not shown again

#### Scenario: Clean working tree

- **WHEN** the loaded status reports no staged, unstaged, untracked or
  conflicted entries
- **THEN** the panel reports that the working tree is clean

#### Scenario: Failed status is an error

- **WHEN** reading the repository status fails
- **THEN** the panel shows the failure
- **AND** it does not report a clean working tree

### Requirement: Branch header

When a status has loaded and the repository is on a branch, the panel SHALL show
the branch name. It SHALL show the ahead count only when it is greater than
zero, and the behind count only when it is greater than zero, each marked as
ahead or behind. When HEAD is detached, no branch name SHALL be shown.

#### Scenario: Branch with divergence

- **WHEN** the repository is on a branch that is 2 ahead and 1 behind its
  upstream
- **THEN** the header shows the branch name, an ahead count of 2, and a behind
  count of 1

#### Scenario: Branch in sync

- **WHEN** the repository is on a branch level with its upstream
- **THEN** the header shows the branch name and neither count

#### Scenario: Detached HEAD

- **WHEN** HEAD is detached
- **THEN** the header shows no branch name

### Requirement: Working tree sections

The working tree SHALL be presented as up to four labelled sections, in this
order: staged changes, unstaged changes, untracked files, and conflicts. A
section SHALL be omitted entirely when it has no entries. Each section SHALL
show its entry count. The whole list SHALL scroll as one region.

The sections are independent filters over the same status, not a partition. A
path with both a staged and an unstaged change SHALL therefore appear once in
the staged section and once in the unstaged section, as two independently
selectable and independently actionable rows.

#### Scenario: Sections appear in order

- **WHEN** the status has staged, unstaged, untracked and conflicted entries
- **THEN** the sections are shown in the order staged, unstaged, untracked,
  conflicts, each with its entry count

#### Scenario: Empty sections are omitted

- **WHEN** the status has staged entries but no untracked entries
- **THEN** no untracked section is shown

#### Scenario: A path staged and modified appears twice

- **WHEN** a path has both a staged change and a later unstaged change
- **THEN** it appears as one row in the staged section and one row in the
  unstaged section

### Requirement: File rows

Each row SHALL show a status glyph for its change type, the file name, and the
containing directory when the path has one. The glyph SHALL reflect the change
type for the side of the status the row represents: a staged row shows the
staged change type, an unstaged, untracked or conflicted row shows the unstaged
one.

**Divergence from Swift**: the Swift reference derives one glyph per path,
preferring the staged change type, so the unstaged row of a staged-and-modified
path shows the staged glyph. Here each row shows its own side's change type,
which is what the row is about.

#### Scenario: Row shows its own side's change type

- **WHEN** a path is staged as added and unstaged as modified
- **THEN** its staged row shows the added change type
- **AND** its unstaged row shows the modified change type

#### Scenario: Row shows name and directory

- **WHEN** a row is for a path nested in a directory
- **THEN** the row shows the file name and the containing directory

#### Scenario: Top-level path shows no directory

- **WHEN** a row is for a path at the repository root
- **THEN** the row shows the file name and no directory

### Requirement: Per-path staging actions

Each row SHALL offer the actions valid for it, and only those:

- A row that is not staged SHALL offer stage.
- A row that is staged SHALL offer unstage.
- A row that is neither staged nor untracked SHALL offer discard.

There is deliberately no action that deletes an untracked file: discard restores
tracked content, and offering it on an untracked row would either do nothing or
destroy a file the user never committed. This is carried over from the Swift
reference unchanged.

Each action SHALL apply to that row's path alone.

#### Scenario: Unstaged tracked row

- **WHEN** a row is a tracked file with unstaged changes
- **THEN** it offers stage and discard, and not unstage

#### Scenario: Staged row

- **WHEN** a row is in the staged section
- **THEN** it offers unstage, and neither stage nor discard

#### Scenario: Untracked row

- **WHEN** a row is an untracked file
- **THEN** it offers stage, and neither unstage nor discard

### Requirement: Discard is confirmed

Discard SHALL require a confirmation naming the path before it runs, and the
confirmation SHALL present discarding as the destructive choice. Cancelling
SHALL leave the working tree untouched.

Staging, unstaging and their bulk forms SHALL NOT be confirmed: each is
reversible by its opposite.

**Divergence from Swift**: the Swift reference discards on a single click of a
hover-revealed icon with no confirmation. Discard throws away work that was
never committed and that nothing can recover, and the panel exists precisely to
review work an agent did that the user has not yet read. An unguarded misclick
there is unrecoverable, so the port guards it.

#### Scenario: Discard asks first

- **WHEN** a row's discard action is activated
- **THEN** a confirmation naming that path is presented
- **AND** no git command has run yet

#### Scenario: Cancelling a discard changes nothing

- **WHEN** the discard confirmation is cancelled
- **THEN** the working tree is unchanged

#### Scenario: Staging is not confirmed

- **WHEN** a row's stage or unstage action is activated
- **THEN** it runs without a confirmation

### Requirement: Bulk staging actions

The staged section SHALL offer an unstage-all action, and the unstaged section
SHALL offer a stage-all action. The untracked and conflicts sections SHALL offer
no bulk action.

Stage-all stages every change in the working tree, including untracked files —
it is `git add -A`, not a stage of only the unstaged section's rows. Because the
untracked section has no bulk control of its own, this is the only action that
stages untracked files in bulk.

#### Scenario: Unstage all

- **WHEN** the staged section's unstage-all action is activated
- **THEN** every staged path is unstaged

#### Scenario: Stage all covers untracked files

- **WHEN** the unstaged section's stage-all action is activated and the working
  tree also has untracked files
- **THEN** the untracked files are staged along with the unstaged changes

#### Scenario: No bulk action on untracked or conflicts

- **WHEN** the untracked or conflicts section is shown
- **THEN** it offers no bulk action

### Requirement: File selection

Selection SHALL identify a row by the pair of its path and whether that row is
the staged side, so the two rows of a staged-and-modified path are selected
independently and show different diffs. The selected row SHALL be visibly
marked.

When a refresh produces a status in which the selected pair no longer exists,
the selection and its diff SHALL be cleared.

**Divergence from Swift**: the Swift reference invalidates the selection only
when the path disappears entirely, so a path that moves from one side to the
other keeps a diff that no longer describes it.

#### Scenario: The two sides select independently

- **WHEN** a staged-and-modified path's staged row is selected and then its
  unstaged row is selected
- **THEN** the unstaged row becomes the selected one
- **AND** the diff shown is the unstaged diff for that path

#### Scenario: Selection survives a refresh that keeps it

- **WHEN** a refresh returns a status that still contains the selected path on
  the selected side
- **THEN** the selection is retained

#### Scenario: Selection is cleared when its side disappears

- **WHEN** the selected row was a path's unstaged side and a refresh returns a
  status where that path has only a staged change
- **THEN** the selection and the displayed diff are cleared

### Requirement: Diff pane

The panel SHALL show the diff for the selected row below the working tree list,
in a region whose split with the list is adjustable by the user. With no
selection it SHALL invite the user to select a file.

When a row is selected, the diff pane SHALL show the file's path and its
addition and deletion counts, each count shown only when greater than zero, and
then the unified diff.

The diff SHALL be for the selected row's side: the staged diff for a staged row,
the working-tree diff otherwise.

#### Scenario: No selection

- **WHEN** no row is selected
- **THEN** the diff pane invites the user to select a file

#### Scenario: Diff header

- **WHEN** a row whose diff has 7 additions and no deletions is selected
- **THEN** the pane shows the file's path and an addition count of 7
- **AND** it shows no deletion count

#### Scenario: Staged row shows the staged diff

- **WHEN** a staged row is selected
- **THEN** the diff shown is the index-against-HEAD diff, not the working-tree
  diff

### Requirement: Diff rendering

Each diff line SHALL be rendered monospaced, on one line, with its old and new
line numbers where the line has them, and marked by its classification as an
addition, a deletion, context, or a hunk header. A hunk header SHALL be rendered
in sequence with the lines it introduces and SHALL show no line numbers.

A diff line SHALL NOT wrap. A wrapped diff line breaks the column alignment that
makes a diff readable and desynchronises it from its line-number gutter.

A binary file SHALL be reported as binary instead of a diff. A diff with no
hunks SHALL report that there are no changes rather than rendering an empty
pane.

#### Scenario: Line classification

- **WHEN** a hunk contains an added line, a removed line and a context line
- **THEN** each is visibly marked as added, removed, or context

#### Scenario: Line numbers

- **WHEN** an added line is rendered
- **THEN** it shows a new line number and no old line number
- **WHEN** a context line is rendered
- **THEN** it shows both

#### Scenario: Hunk header

- **WHEN** a hunk header line is rendered
- **THEN** it is visibly distinct from the lines it introduces
- **AND** it shows no line numbers

#### Scenario: Long lines do not wrap

- **WHEN** a diff line is wider than the diff pane
- **THEN** it remains on one line and the pane scrolls horizontally

#### Scenario: Binary file

- **WHEN** the selected file's diff is binary
- **THEN** the pane reports a binary file and renders no diff lines

#### Scenario: Diff with no hunks

- **WHEN** the selected file's diff has no hunks and is not binary
- **THEN** the pane reports that there are no changes

### Requirement: Large diffs are bounded

The panel SHALL bound the work a single frame does for a diff, so that selecting
a very large diff does not make the window's frame time scale with the size of
the diff. When a diff is too large to present in full, the panel SHALL render a
bounded prefix of it and SHALL state that it has been truncated, rather than
silently showing part of a file as though it were all of it.

**Divergence from Swift**: the Swift reference renders every line of every hunk
with no cap, which is affordable there because its view layer does not rebuild
the whole element tree per frame.

#### Scenario: A large diff does not scale frame cost

- **WHEN** a file with a very large diff is selected
- **THEN** the panel remains responsive
- **AND** the work done per frame does not grow with the number of diff lines

#### Scenario: Truncation is stated

- **WHEN** a diff is too large to present in full
- **THEN** the panel shows a bounded prefix and states that the diff was
  truncated

### Requirement: Commit

A commit control SHALL be offered only while something is staged, and SHALL open
a prompt for the commit message. The prompt SHALL accept a multi-line message
and SHALL offer confirm and cancel.

Confirm SHALL be unavailable while the message is empty or only whitespace, and
while a commit is already in flight. The committed message SHALL be the entered
message trimmed of leading and trailing whitespace; internal blank lines SHALL
be preserved, so the subject-blank-line-body convention survives.

On success the prompt SHALL be dismissed and the panel SHALL refresh. On failure
the prompt SHALL remain, SHALL show the failure, and SHALL retain the entered
message so it need not be retyped.

Only `git commit -m <message>` is in scope. Amending, author overrides and
signing flags are deliberately not offered.

#### Scenario: Commit control requires staged content

- **WHEN** nothing is staged
- **THEN** no commit control is offered
- **WHEN** at least one path is staged
- **THEN** the commit control is offered

#### Scenario: Whitespace-only message cannot be committed

- **WHEN** the commit message is empty or contains only whitespace
- **THEN** confirm is unavailable

#### Scenario: Message is trimmed but its body preserved

- **WHEN** a message with surrounding blank lines and an internal blank line
  separating subject from body is committed
- **THEN** the surrounding whitespace is removed and the internal blank line is
  kept

#### Scenario: Successful commit

- **WHEN** the commit succeeds
- **THEN** the prompt is dismissed and the panel refreshes

#### Scenario: Failed commit keeps the message

- **WHEN** the commit fails
- **THEN** the prompt remains, shows the failure, and still contains the
  entered message

### Requirement: Refresh

The panel SHALL reload the repository status when it opens, when its refresh
control is used, after any staging action it performs succeeds, and when a
relevant change is seen in the working tree.

A staging action that fails SHALL surface the failure. The panel SHALL NOT
present the tree as though the failed action had been applied.

#### Scenario: Refresh after a successful action

- **WHEN** a stage, unstage, discard or bulk action succeeds
- **THEN** the panel reloads the repository status

#### Scenario: A failed action surfaces

- **WHEN** a staging action fails
- **THEN** the panel shows the failure and does not show the tree as though it
  had been applied

### Requirement: The panel's own writes do not trigger it

While the panel performs a git operation, its working-tree watch SHALL be
suppressed and SHALL resume only after a settle delay past the operation, so the
panel's own writes do not trigger the refresh that follows them.

#### Scenario: Own stage does not self-trigger

- **WHEN** the panel stages a path and refreshes
- **THEN** the filesystem changes caused by that stage do not themselves trigger
  a further refresh

#### Scenario: An external change still refreshes

- **WHEN** a file in the working tree is changed outside Knot while the panel is
  open and idle
- **THEN** the panel refreshes

### Requirement: Git work stays off the render path

No render SHALL run a git command. A render SHALL draw the most recent status
and diff it already has, and request work whose result a later frame draws. The
same request SHALL NOT be issued repeatedly while one is outstanding.

This is the rule that `crates/knot/src/diff_stats.rs` exists to enforce for the
dashboard cards; the panel issues far more git calls than a card does, so it
holds here at least as strongly.

#### Scenario: Rendering issues no git command

- **WHEN** the panel is rendered repeatedly, for instance while typing in the
  commit message field
- **THEN** no git command is run as a result of those renders

#### Scenario: One request per outstanding refresh

- **WHEN** a status refresh is outstanding and further renders occur
- **THEN** no additional status refresh is requested for the same panel

#### Scenario: Only the current selection's diff is shown

- **WHEN** one row is selected and another is selected before the first diff
  arrives
- **THEN** the diff eventually shown is the second row's
- **AND** the first row's diff is never shown

### Requirement: Diff stats follow the panel

After the panel performs a git operation that changes the working tree, the
agent's cached diff stats SHALL be invalidated so the agent's dashboard card
reflects the change without waiting out the normal staleness interval.

#### Scenario: Commit updates the card

- **WHEN** the panel commits every staged change for an agent whose card is
  showing diff stats
- **THEN** the card's stats are refreshed rather than continuing to show the
  pre-commit counts until they age out
