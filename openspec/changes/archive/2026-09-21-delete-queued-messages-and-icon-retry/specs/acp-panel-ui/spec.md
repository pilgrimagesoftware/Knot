# Spec Delta

## ADDED Requirements

### Requirement: Retry uses the icon-button convention

The retry action SHALL render as an icon button rather than a text button. The icon SHALL have a localized accessible label and tooltip describing retry, and activating it SHALL preserve the existing retry behavior.

#### Scenario: Retry renders as an icon
- **WHEN** a response exposes a retry action
- **THEN** the action is shown as an icon button with no visible text label

#### Scenario: Retry tooltip identifies the action
- **WHEN** the user points to or focuses the retry icon
- **THEN** a localized tooltip and accessible label identify it as retry

#### Scenario: Retry behavior is unchanged
- **WHEN** the user activates the retry icon
- **THEN** the same message is retried using the existing retry flow
