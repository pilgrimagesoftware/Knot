# Spec Delta

## ADDED Requirements

### Requirement: Queued messages use compact single-line typography

The panel SHALL render a queued message's visible content in the theme's
monospace font on one line. If the content exceeds the available row width,
the panel SHALL truncate it with an ellipsis rather than wrapping it. Other
message types SHALL retain their existing layout and typography.

#### Scenario: Long queued message is truncated

- **WHEN** a queued message is wider than the available row
- **THEN** its content remains on one line, uses the monospace font, and ends
  with an ellipsis

#### Scenario: Short queued message fits

- **WHEN** a queued message fits within the available row
- **THEN** its full content is shown on one line in the monospace font

### Requirement: Failed status uses failure color and proportional typography

When a queued message or message status is `failed`, the panel SHALL render
the status label in the theme's failure color and proportional font. The
status label SHALL remain visually distinct from the monospace message
content.

#### Scenario: Failed queued message is marked

- **WHEN** a queued message reaches `failed`
- **THEN** its status label uses the theme's failure color and proportional
  font while the message content remains monospace

#### Scenario: Non-failed status is unchanged

- **WHEN** a queued message has a status other than `failed`
- **THEN** its status uses the existing status styling and does not use the
  failure color solely because the message is queued
