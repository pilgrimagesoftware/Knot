## Purpose

Delivers keyboard and mouse input from the visible terminal surface to its
underlying PTY session, including IME composition, so the terminal is
actually usable rather than merely visible.

## ADDED Requirements

### Requirement: Keyboard input reaches the focused terminal surface

While a terminal surface has keyboard focus, key presses (including
modifier-only changes) SHALL be delivered to that surface's session, with
the same key producing the same terminal behavior a native terminal app
would produce (control sequences, printable text, function keys).

#### Scenario: Typed text appears in the terminal

- **WHEN** the terminal surface has focus and the user types printable
  characters
- **THEN** those characters are sent to the agent's PTY session as input

#### Scenario: Control sequences are sent, not the raw key

- **WHEN** the user presses a control combination (e.g. Ctrl+C) with the
  terminal focused
- **THEN** the corresponding control sequence is delivered to the PTY, not
  the literal character

### Requirement: IME composition works in the terminal

Composing input via the system input method (e.g. Pinyin, Kana) into a
focused terminal surface SHALL show the in-progress composition at the
correct on-screen position and SHALL commit only the composed result to
the PTY, not intermediate keystrokes.

#### Scenario: CJK composition commits the composed text

- **WHEN** the user composes text via an IME with the terminal focused
- **THEN** the candidate window appears anchored at the terminal cursor's
  screen position
- **AND** only the final committed text is sent to the PTY session, not the
  raw keys that produced it

### Requirement: Mouse input reaches terminal-aware programs

Mouse clicks, drags, and scroll events over a focused terminal surface
SHALL be delivered to it when the running program has requested mouse
reporting (e.g. an interactive TUI), and SHALL fall back to normal
scrollback/selection behavior otherwise.

#### Scenario: Mouse-aware TUI receives clicks

- **WHEN** a program that enables terminal mouse reporting is running in
  the surface and the user clicks within it
- **THEN** the click is delivered to the program, not treated as a text
  selection

#### Scenario: Scrolling without mouse reporting scrolls the view

- **WHEN** no program has enabled mouse reporting and the user scrolls over
  the surface
- **THEN** the terminal's own scrollback view scrolls
