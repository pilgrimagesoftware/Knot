# Spec Delta

## MODIFIED Requirements

### Requirement: A preference change applies to open windows

A preference written through the settings surface SHALL take effect in every
already-open window that draws from it, without that window being closed and
reopened, and without a delivery step that each window must opt into.

The running system SHALL hold one settings surface. A window SHALL NOT hold a
copy that can diverge from it: a window reading a preference after it was
written SHALL observe the written value regardless of which window wrote it,
which window is reading, or when either was opened.

Reading the settings surface SHALL be safe on the render path, which is
re-entered on every frame. A read SHALL NOT block on a write and SHALL NOT
block on another read.

A reader that has taken the surface SHALL continue to see a consistent set of
values for as long as it holds it, even if a write lands meanwhile. A write
SHALL NOT be observable as a partially-applied set of values.

#### Scenario: Compact tool calls applies to an open panel

- **WHEN** a workspace window is open and the user enables compact tool-call
  mode in the settings window
- **THEN** that window's panel draws the summary line without being reopened

#### Scenario: A non-workspace window sees the change too

- **WHEN** the import window is open and a preference it draws is changed in
  the settings window
- **THEN** the import window draws the new value without being reopened

#### Scenario: Refresh keeps the roster

- **WHEN** a preference is written while the in-memory roster differs from
  what the roster documents on disk hold
- **THEN** the scalar preferences reflect the write and the roster is
  unchanged, in memory and on disk

#### Scenario: A window opened later is unaffected

- **WHEN** a preference is changed and a workspace window is opened afterwards
- **THEN** that window reads the changed value, as it did before

#### Scenario: A reader mid-frame sees one consistent set

- **WHEN** a write lands while a window is part-way through reading several
  preferences for one frame
- **THEN** that frame draws every value as it stood at the read, and the next
  frame draws every value as it stands after the write

## ADDED Requirements

### Requirement: A write preserves values written elsewhere

Writing through the settings surface SHALL persist the values the writer
changed and SHALL preserve every value the writer did not change, including
values written by another part of the system since the writer began.

No holder of the settings surface SHALL be able to revert a value it never
set. This applies to the scalars and to the durable collections alike, and it
applies however long the holder has been open.

#### Scenario: An import does not revert a preference

- **WHEN** the user opens the import window, changes a preference in the
  settings window, and then completes an import
- **THEN** the imported records are stored and the changed preference still
  reads as the user set it

#### Scenario: A roster write does not revert a preference

- **WHEN** a preference is changed and a window that has been open since
  before the change then adds, renames or removes an agent
- **THEN** the roster change is stored and the changed preference still reads
  as the user set it

#### Scenario: Two windows writing different values

- **WHEN** one window writes a preference and another window, open since
  before that write, then writes a different preference
- **THEN** both written values are stored and neither reverts the other
