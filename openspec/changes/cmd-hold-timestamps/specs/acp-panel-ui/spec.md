# Spec Delta

## ADDED Requirements

### Requirement: Message timestamps are shown only while ⌘ is held
A prompt's or response's timestamp label SHALL be hidden by default and SHALL
appear only while the ⌘ key is held and the panel's window is key, matching
the sidebar's existing key-hint disclosure. A hidden timestamp SHALL NOT
reserve layout space.

#### Scenario: Timestamps hidden by default
- **WHEN** a conversation is showing and no modifier key is held
- **THEN** no prompt or response in the panel shows a timestamp label

#### Scenario: Holding ⌘ reveals timestamps after a short delay
- **WHEN** the user holds ⌘ while the panel's window is key, for at least
  the sidebar key-hint delay
- **THEN** every visible prompt and response shows its relative timestamp,
  each with a tooltip giving the absolute time in the user's local time zone

#### Scenario: Releasing ⌘ hides timestamps immediately
- **WHEN** timestamps are shown because ⌘ is held
- **AND** the user releases ⌘
- **THEN** every timestamp label disappears immediately

#### Scenario: A keypress during the hold cancels it
- **WHEN** ⌘ is held and timestamps are showing or pending
- **AND** any other key is pressed
- **THEN** the hold ends and timestamps hide, matching the sidebar key-hint
  behavior for an in-progress shortcut

#### Scenario: Losing key window status ends the hold
- **WHEN** the panel's window stops being the active key window while ⌘ is
  held
- **THEN** the hold ends and timestamps hide, since macOS does not deliver
  the ⌘ key-up to a window that lost key status
