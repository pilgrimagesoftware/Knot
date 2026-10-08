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
the agent's config option value (e.g. containing "bypass", "yolo",
"danger" or "full access" maps to the highest risk level; containing
"plan" or "read" maps to the lowest). Hyphens and underscores count as
spaces, so `agent-full-access` matches "full access". A mode whose identifier matches no known risk keyword
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

#### Scenario: Codex full-access mode is colored as dangerous
- **WHEN** the agent's current permission-mode config option value is
  `agent-full-access`, Codex's counterpart to `bypassPermissions`
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

### Requirement: The prompt offers Always Allow when the agent does

When the options of a permission request carry kinds, the prompt SHALL show
one control per option, in the order the agent sent them and labelled with
the agent's name for each. Allow, Always Allow and Deny SHALL each be bound
to a key: Allow and Deny as before, and Always Allow to its own. Each
control's key hint SHALL be on the option that key answers with.

An Always Allow key pressed on a request that offers no `allow_always` option
SHALL leave the request pending.

When no option carries a kind, the prompt SHALL keep its Allow and Deny
controls.

#### Scenario: Always Allow is offered

- **WHEN** a permission request offers Always Allow, Allow and Reject
- **THEN** the prompt shows all three, each by the agent's name for it, and
  Always Allow carries its key's hint

#### Scenario: Keyboard Always Allow

- **WHEN** a permission request offering Always Allow is pending and the
  user presses the Always Allow key
- **THEN** the request is answered with the Always Allow option

#### Scenario: Always Allow not offered

- **WHEN** a pending request offers no Always Allow option and the user
  presses the Always Allow key
- **THEN** the request stays pending
