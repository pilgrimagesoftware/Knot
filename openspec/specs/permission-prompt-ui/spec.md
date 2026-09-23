# permission-prompt-ui Specification

## Purpose
Gives the agent panel's permission-mode selector and inline permission
prompt a risk-colored appearance and full keyboard operability, so a user
can see at a glance when a mode removes guardrails and can act on a
pending permission decision without a mouse.

## Requirements

### Requirement: Permission-mode selector is colored by risk level
The agent panel's permission-mode selector SHALL render with a color that
reflects the risk level of the currently selected mode, and SHALL apply
that same color to each mode's entry in the selector's dropdown list.

Risk level is derived from the mode's identifier or name as reported by
the agent's config option value (e.g. containing "bypass", "yolo", or
"danger" maps to the highest risk level; containing "plan" or "read"
maps to the lowest). A mode whose identifier matches no known risk keyword
renders with the selector's default (unstyled) appearance.

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

### Requirement: Inline permission prompt is colored by the active permission mode's risk level
The inline permission prompt SHALL render its accent (border) color
according to the risk level of the agent's currently active permission
mode (the same config option value driving the selector), when the agent
has declared a permission-mode config option, and SHALL fall back to the
existing neutral accent color when it has not.

#### Scenario: Prompt reflects a high-risk active mode
- **WHEN** a permission request is pending and the agent's active
  permission mode matches a high-risk keyword
- **THEN** the permission prompt's border renders in the high-risk color

#### Scenario: Prompt falls back with no mode context
- **WHEN** a permission request is pending and the agent has not declared
  a permission-mode config option
- **THEN** the permission prompt's border renders in the existing default
  neutral color

### Requirement: Permission prompt is keyboard-operable
While an inline permission prompt is visible, the user SHALL be able to
allow or deny it via a keybinding, in addition to the existing click
targets, without needing to move focus away from the panel.

Each of the prompt's decision buttons SHALL display the keystroke bound to
its action, read from the installed keymap so the hint cannot disagree with
the binding that fires. A button whose action has no binding installed
SHALL render exactly as it did before, with no hint and no placeholder.

#### Scenario: Keyboard allow
- **WHEN** an inline permission prompt is visible and the user invokes the
  allow keybinding
- **THEN** the pending request is resolved with an allow decision, the
  same as clicking the "Allow" button

#### Scenario: Keyboard deny
- **WHEN** an inline permission prompt is visible and the user invokes the
  deny keybinding
- **THEN** the pending request is resolved with a deny decision, the same
  as clicking the "Deny" button

#### Scenario: No pending prompt ignores the keybinding
- **WHEN** no inline permission prompt is visible and the user invokes the
  allow or deny keybinding
- **THEN** no permission decision is sent and no error occurs

#### Scenario: Buttons show the keystroke that drives them
- **WHEN** an inline permission prompt is visible and the allow and deny
  actions each have a keybinding installed
- **THEN** the "Allow" and "Deny" buttons each display their own bound
  keystroke, formatted for the platform, beside the label

#### Scenario: An unbound action shows no hint
- **WHEN** an inline permission prompt is visible and one of the decision
  actions has no keybinding installed
- **THEN** that button renders with its label alone, and the other button's
  hint is unaffected

### Requirement: Permission-mode selector is keyboard-operable
While the panel input area has focus, the user SHALL be able to open the
permission-mode selector's dropdown via a keybinding, in addition to
clicking the selector button.

#### Scenario: Keyboard opens the mode dropdown
- **WHEN** the panel input area has focus and the user invokes the
  open-permission-selector keybinding
- **THEN** the permission-mode selector's dropdown opens, showing the same
  entries as a mouse click would
