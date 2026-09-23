# Spec Delta

## MODIFIED Requirements

### Requirement: Permission-mode selector is colored by risk level
The agent panel's permission-mode selector SHALL render with a color that
reflects the risk level of the currently selected mode, and SHALL apply
that same color to each mode's entry in the selector's dropdown list.

Risk level is derived from the mode's identifier or name as reported by
the agent's config option value (e.g. containing "bypass", "yolo", or
"danger" maps to the highest risk level; containing "plan" or "read"
maps to the lowest). A mode whose identifier matches no known risk keyword
renders with the selector's default (unstyled) appearance.

The mode's option SHALL be located without requiring the agent to have
categorized it, under the same matching rule the input area's selectors
use. Whether the agent labelled the option with a category SHALL NOT
decide whether the coloring appears.

#### Scenario: Bypass-permissions mode is colored as dangerous
- **WHEN** the agent's current permission-mode config option value is
  `bypassPermissions` (or another value matching a high-risk keyword)
- **THEN** the permission-mode selector button renders in the high-risk
  (warning/red) color

#### Scenario: Restricted mode is colored as safe
- **WHEN** the agent's current permission-mode config option value
  matches a low-risk keyword (e.g. `plan`)
- **THEN** the permission-mode selector button renders in the low-risk
  (safe/neutral-green) color

#### Scenario: Unrecognized mode falls back to default styling
- **WHEN** the agent's current permission-mode config option value
  matches no known risk keyword
- **THEN** the permission-mode selector button renders with its default,
  unstyled appearance

#### Scenario: Dropdown entries match their own risk color
- **WHEN** the user opens the permission-mode selector's dropdown
- **THEN** each listed mode is colored according to its own risk level,
  independent of the currently selected mode's color

#### Scenario: An uncategorized mode option is still colored
- **WHEN** the agent declares its permission-mode option with no category,
  and the option's current value matches a known risk keyword
- **THEN** the selector renders in that risk level's color, the same as if
  the agent had categorized the option

### Requirement: Inline permission prompt is colored by the active permission mode's risk level
The inline permission prompt SHALL render its accent (border) color
according to the risk level of the agent's currently active permission
mode (the same config option value driving the selector), when the agent
has declared a permission-mode config option, and SHALL fall back to the
existing neutral accent color when it has not.

An agent that declared the option but attached no category to it counts as
having declared it. The neutral fallback SHALL mean the agent declared no
such option, never that the system failed to recognize one it did declare.

#### Scenario: Prompt reflects a high-risk active mode
- **WHEN** a permission request is pending and the agent's active
  permission mode matches a high-risk keyword
- **THEN** the permission prompt's border renders in the high-risk color

#### Scenario: Prompt falls back with no mode context
- **WHEN** a permission request is pending and the agent has not declared
  a permission-mode config option
- **THEN** the permission prompt's border renders in the existing default
  neutral color

#### Scenario: An uncategorized mode option still colors the prompt
- **WHEN** a permission request is pending, the agent declared its
  permission-mode option with no category, and its active value matches a
  high-risk keyword
- **THEN** the permission prompt's border renders in the high-risk color
  rather than the neutral fallback

### Requirement: Permission-mode selector is keyboard-operable
While a Panel-mode agent is the selected agent, the user SHALL be able to
open the permission-mode selector's dropdown via a keybinding, in addition
to clicking the selector button. The keybinding SHALL NOT require the
panel input area to hold keyboard focus.

#### Scenario: Keyboard opens the mode dropdown
- **WHEN** a Panel-mode agent is selected and the user invokes the
  open-permission-selector keybinding
- **THEN** the permission-mode selector's dropdown opens, showing the same
  entries as a mouse click would

#### Scenario: The keybinding does not need input focus
- **WHEN** a Panel-mode agent is selected, focus sits elsewhere in the
  pane rather than in the input area, and the user invokes the
  open-permission-selector keybinding
- **THEN** the permission-mode selector's dropdown still opens
