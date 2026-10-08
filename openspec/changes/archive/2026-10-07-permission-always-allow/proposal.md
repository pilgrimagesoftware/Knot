# Proposal

## Why

Issue #530: agents offer an "always allow" choice on permission requests, and
Knot's prompt shows only Allow and Deny.

The reason is worse than a missing button. ACP permission options each carry
a `kind` (`allow_once`, `allow_always`, `reject_once`, `reject_always`), and
`knot-acp` dropped it when parsing. A decision was then turned into an option
by position and wording:

- **Allow** sent the first option. Claude's ACP adapter lists Always Allow
  first, so every Allow click granted the tool for the rest of the session.
  On the exit-plan-mode prompt, the first option also switches the session to
  auto-accept edits.
- **Deny** looked for "deny" in the option names. The adapter says "Reject",
  so Deny answered with an option id the agent never offered.

## What Changes

- **`knot-acp`:** parses each option's `kind` (an unknown value leaves that
  option kindless, never empties the list). Each decision resolves to the
  option of its kind:
  - Allow → `allow_once`
  - Always Allow → `allow_always`
  - Deny → `reject_once`, else `reject_always`
  - a direct choice → the option chosen

  A decision whose kind isn't offered picks nothing. An adapter that sends no
  kinds is answered exactly as before.
- **`knot` prompt:** one control per offered option, in the agent's order,
  labelled with the agent's wording, as `acp-panel-ui` already requires. Each
  decision key's hint sits on the option it answers with. Without kinds, the
  prompt keeps the Allow and Deny pair it had.
- **New key:** Always Allow (`cmd-alt-shift-a`), beside Allow
  (`cmd-shift-a`) and Deny (`cmd-shift-d`). A key whose kind the request
  doesn't offer leaves the request pending.

## Capabilities

### Modified Capabilities

- `acp-client`: permission options carry their kind, and decisions resolve by
  kind.
- `permission-prompt-ui`: Always Allow is offered and keyboard-operable when
  the agent offers it.

## Impact

- `crates/knot-acp/src/protocol/permission.rs` (new; the permission types
  move out of `protocol/mod.rs`) and `client/mod.rs`'s `permission_result`.
- `crates/knot/src/panel_view/render.rs` (`prompt_choices`), the keymap, the
  action list, and the selected-agent answer path.
