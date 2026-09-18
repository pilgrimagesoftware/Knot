## Why

An agent row's detail lines are half labelled. Its agent type and its
companion marker each carry an icon; its status and its folder are bare
strings, and its persona carries an emoji typed into the text rather than a
drawn icon.

The result reads as a list of four unrelated strings where two of them happen
to have pictures. A row showing `Ready — standing…` above `WIP` gives the
reader nothing to say which is the status and which is the directory beyond
guessing from the words - and the guess fails on an agent whose status
mentions a path, or whose folder is named after a person.

## What Changes

- **The status line gets an icon**, marking it as the agent's status.
- **The folder line gets an icon**, marking it as a directory.
- **The persona's emoji becomes a drawn icon.** This goes slightly beyond the
  request and is what makes the rest of it work: with status and folder
  gaining muted, line-sized icons, a full-colour emoji sitting inline at a
  different size leaves the column ragged - the problem the change is meant
  to fix, one line further down.
- **Each icon is sized to its own line.** The row's lines do not all render
  at the same text size, so one fixed icon size is wrong on some of them.

## Capabilities

### Modified Capabilities
- `agent-list-ui`: adds what an agent row's detail lines carry.

### New Capabilities
(none)

## Impact

- `crates/knot`: the agent row's detail lines in `workspace_window`.
- Nothing else. No new data - every line already renders; this is what each
  one is labelled with.
