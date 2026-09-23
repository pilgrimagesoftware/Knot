# Spec Delta

## MODIFIED Requirements

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
