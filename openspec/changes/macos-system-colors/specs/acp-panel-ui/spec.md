# Spec Delta

## ADDED Requirements

### Requirement: Panel chrome adopts the macOS system palette
On macOS, the panel's chrome SHALL follow the platform's system colors: the
user prompt bubble and primary controls tinted by the system accent color
(`controlAccentColor`), the prompt's foreground contrasting the resolved
accent, focused controls bordered/tinted by that accent, and neutral surfaces
(bubbles, cards, inputs, separators, window background) taken from the
system's dynamic neutrals so they track the current appearance. Non-macOS
platforms SHALL render the fixed palette they render today.
Semantic state colors (agent status indicators, risk tinting, danger) SHALL
NOT be replaced by system colors.

#### Scenario: The prompt bubble follows the user's accent color
- **WHEN** the system accent color is changed on macOS
- **THEN** the user prompt bubble and primary buttons render in that accent,
  and the prompt text remains readable against it

#### Scenario: The prompt stays readable in light mode
- **WHEN** the panel is in light appearance with an accent whose luminance
  makes white text unreadable
- **THEN** the prompt foreground switches to a dark contrast color

#### Scenario: Focus paints with the system accent
- **WHEN** an input or button receives focus on macOS
- **THEN** its border and focus tint use the system accent, not a fixed
  application blue

#### Scenario: An appearance flip repaints live
- **WHEN** the macOS appearance changes from light to dark (or back)
- **THEN** the panel's neutral surfaces re-resolve and repaint without
  requiring a settings change or restart

#### Scenario: State colors stay fixed
- **WHEN** a tool call or agent renders a semantic state color (idle,
  running, error, safe, danger)
- **THEN** that color is the application's fixed state palette, unchanged by
  the system accent or appearance

#### Scenario: Non-macOS platforms are unchanged
- **WHEN** the app runs on Linux or Windows
- **THEN** the panel renders with the existing fixed palette