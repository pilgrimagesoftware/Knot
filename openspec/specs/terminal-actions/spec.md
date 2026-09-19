# terminal-actions Specification

## Purpose
Routes events the terminal engine originates (not user input) - title
changes and clipboard requests - from a surface into the rest of the app.

## Requirements

### Requirement: Terminal title changes update the agent's displayed title

When the running program sets the terminal title (e.g. via shell
integration or an explicit escape sequence), the agent's `terminal_title`
SHALL update, following the same precedence `Agent::header_title` already
defines (status text overrides terminal title).

#### Scenario: Shell sets a title

- **WHEN** the program running in an agent's terminal sets the terminal
  title
- **THEN** that agent's `terminal_title` is updated
- **AND** the sidebar/header reflects it unless a `status_text` is set

### Requirement: Clipboard requests are served from the OS pasteboard

A terminal surface's read-clipboard and write-clipboard requests SHALL be
served from the OS pasteboard, applying the same configurable text
transforms the Swift reference applies on copy (strip ANSI codes, trim
trailing whitespace) by default.

#### Scenario: Copying terminal selection strips ANSI codes by default

- **WHEN** the user copies a text selection containing ANSI escape codes
  from a terminal surface
- **THEN** the OS pasteboard receives the text with ANSI codes stripped

#### Scenario: Pasting into the terminal reads the OS pasteboard

- **WHEN** the running program requests the clipboard (e.g. via a paste
  keybinding)
- **THEN** the current OS pasteboard contents are delivered to it
