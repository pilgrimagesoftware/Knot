# Spec Delta

## ADDED Requirements

### Requirement: The conversation shows that a turn is in progress

While a turn is active the conversation SHALL show a turn-in-progress indicator
as its last row, below every message and below the permission prompt and
ended-session banner when either is present. The row SHALL appear when the turn
becomes active and SHALL be gone once the turn ends, so its presence is itself
the statement that the agent is still answering.

The indicator SHALL be an animated rendering of the application's own icon. It
SHALL NOT be the dashboard card's working indicator: that mark exists to
distinguish four agent states in a dense grid, and this row has one thing to
say. This diverges from the Swift reference, whose panel has no indicator of
this kind at all.

The animation SHALL be continuous while the turn is active — it SHALL NOT stop
on a frame, run once, or wait for unrelated activity to advance it. A reader
watching a turn that produces no output for several seconds SHALL still see
motion.

The indicator SHALL respect the system's reduced-motion setting: when reduced
motion is in effect the row SHALL still be present and SHALL still show the
icon, held still rather than animating. Presence, not motion, is what carries
the meaning in that case.

The indicator SHALL carry no text. It SHALL be drawn large enough to read as a
deliberate mark rather than as a stray glyph, and SHALL reserve the same space
whether it is animating or held still, so the conversation does not reflow when
the setting changes.

#### Scenario: A turn is running

- **WHEN** an agent's turn is active
- **THEN** the conversation's last row shows the animated application icon

#### Scenario: The turn ends

- **WHEN** the agent's turn ends
- **THEN** the indicator row is gone and the rows above it are unchanged

#### Scenario: A quiet turn still shows motion

- **WHEN** a turn is active and the agent has produced no output for several
  seconds
- **THEN** the indicator is still animating

#### Scenario: The indicator sits below a permission prompt

- **WHEN** a turn is active and a permission request is awaiting a decision
- **THEN** the permission prompt is rendered above the indicator row

#### Scenario: Reduced motion is in effect

- **WHEN** the system's reduced-motion setting is on and a turn is active
- **THEN** the indicator row is present and shows the icon without animating

#### Scenario: The conversation does not reflow when motion is disabled

- **WHEN** the indicator is shown animating and then shown held still
- **THEN** it occupies the same space in both cases
