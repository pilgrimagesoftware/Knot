## Purpose

Defines the Import window - where an import is run from, what it shows before
it writes anything, and what it reports afterwards.

## ADDED Requirements

### Requirement: Import is its own window, not a setting

Knot SHALL offer its imports in a window of their own, opened from the File
menu, and SHALL NOT place them among the settings panes. An import is an
action the user runs at a moment they choose, not a preference they keep;
settings holds what Knot remembers, and an import changes the store once and
is done.

The window SHALL be a single instance: choosing Import again while it is open
SHALL raise the open window rather than opening a second one, so two windows
cannot offer the same records and import them twice.

The window SHALL read the store as it opens rather than a copy captured
earlier, so an import cannot overwrite a change saved since Knot started.

#### Scenario: Import is reachable from the File menu

- **WHEN** the user opens the File menu
- **THEN** it offers Import…, and choosing it opens the Import window

#### Scenario: Settings holds no import

- **WHEN** the user opens the settings window
- **THEN** no pane offers an import, and no tab is named Import

#### Scenario: Choosing Import twice

- **WHEN** the Import window is open and the user chooses Import… again
- **THEN** the open window is raised, and no second window appears

### Requirement: The window shows every source before it writes

The Import window SHALL hold one section per source Knot can import from:
personas from a coding-agent tool's subagent definitions, and workspaces from
Skwad.

Each section SHALL show what its source holds, let the user select which
records to import, and report the result afterwards - what was added, what
was skipped as already present, and what could not be read - per
`data-import`.

A source with nothing to offer SHALL say so in place of its list, rather than
showing an empty list or being hidden. A hidden section is indistinguishable
from a section that does not exist, and the user cannot tell whether Knot
looked.

The window SHALL scan its sources when it opens and SHALL offer to scan them
again, so a definition written while the window is open can be imported
without reopening it. Scanning SHALL NOT happen while drawing a frame.

#### Scenario: Importing personas from Claude

- **WHEN** the user opens the Import window with Claude subagent definitions
  present
- **THEN** the personas section lists them for selection, and importing the
  selected ones adds them and reports how many were added and skipped

#### Scenario: A source with nothing in it

- **WHEN** the user opens the Import window with no Skwad installation
  present
- **THEN** the Skwad section says there is nothing to import, rather than
  showing an empty list

#### Scenario: Unreadable records are named

- **WHEN** an import finishes with records it could not read
- **THEN** the section names them, rather than reporting a count alone

#### Scenario: A definition written while the window is open

- **WHEN** the user writes a new subagent definition and asks the window to
  look again
- **THEN** the new definition is offered, and any earlier selection is
  cleared rather than pointing at records that may no longer be there

### Requirement: An import always says what it did

Every import SHALL report its outcome on screen, and the report SHALL remain
visible without the user having to scroll or resize to find it. There is no
input for which an import reports nothing: one that added nothing says so,
and one that failed says so with the reason.

A failure SHALL be distinguishable from a success at a glance. An import that
failed and an import that did nothing otherwise read identically, and only
one of them needs acting on.

Reporting SHALL NOT depend on a delivery channel the user may not be able to
see. A message written only to standard output, or a desktop notification
that the platform suppresses outside an app bundle, does not satisfy this.

#### Scenario: An import that fails

- **WHEN** an import cannot complete
- **THEN** the window names the failure and its reason, drawn so it does not
  read as an ordinary summary

#### Scenario: An import with nothing to do

- **WHEN** every selected record is already present
- **THEN** the window says there was nothing to import, rather than showing
  an empty panel

#### Scenario: A summary that will not fit

- **WHEN** the sources hold more records than the window can show at once
- **THEN** the lists scroll and the summary stays in view, rather than the
  summary being pushed out of the window

### Requirement: An import reaches the running application

Imported workspaces and agents SHALL be added to the live store the open
windows render from, not only to the settings document. A record written to
settings alone is invisible until the application is restarted.

Every open window SHALL be scheduled to redraw when an import completes.
Rendering from shared state is not enough on its own: a window that is not
scheduled for a paint goes on showing the frame it last drew, so an import
that correctly updated the store still appeared to have done nothing.

The live copy of a record SHALL win over an incoming one: an agent the
application already holds has session state that an imported copy does not,
so a record whose identifier is already present is left as it is.

Adding to the store SHALL be idempotent, so importing again adds nothing a
second time.

#### Scenario: An imported workspace appears without a restart

- **WHEN** the user imports a workspace while the workspace manager is open
- **THEN** the workspace manager shows it, without the application being
  restarted

#### Scenario: An imported workspace survives the next save

- **WHEN** a workspace is imported and the user then renames, reorders or
  deletes any workspace
- **THEN** the imported workspace is still present afterwards

#### Scenario: An agent already running is not replaced

- **WHEN** an import offers a record whose identifier the application already
  holds
- **THEN** the held record is left exactly as it is, with its session intact
