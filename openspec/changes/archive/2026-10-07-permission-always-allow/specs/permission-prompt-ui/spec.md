# Spec Delta

## ADDED Requirements

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
