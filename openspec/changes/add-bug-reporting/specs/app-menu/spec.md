# Spec Delta

## ADDED Requirements

### Requirement: The Help menu carries a Report a Bug item

In addition to the standard mac-wide table, the Help menu SHALL carry a
"Report a Bug…" item that opens the bug report window per `bug-reporting`.

The item SHALL have no key equivalent: GitHub gives the action none, and the
platform gives none to hand it. It SHALL always be enabled - a report can be
filed from a window in any state, and this is one of the two ways the user can
ask for help.

#### Scenario: Report a Bug opens the bug report window

- **WHEN** the user chooses Help > Report a Bug…
- **THEN** the bug report window opens attached to the active window per
  `bug-reporting`, and the item has no key equivalent displayed

#### Scenario: The item is not a placeholder

- **WHEN** the user opens the Help menu
- **THEN** Report a Bug… is enabled and answers, unlike the standard Knot Help
  item, which stays a disabled placeholder until it is wired