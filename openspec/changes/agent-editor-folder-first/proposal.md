# Proposal

## Why

The new-agent dialog asks for the agent's name first and its folder last, but
the folder is what the dialog actually needs: `can_submit` refuses to enable
the primary button without a folder that exists on disk, and
`validated_fields` checks the folder *before* the name. So the field the form
depends on most is the one the user reaches last, after scrolling past three
other sections.

The name is the second cost. Almost every agent is named after the directory
it works in, and `AgentStore::create` already knows that - given no name it
falls back to the folder's last path component. The dialog can never reach
that fallback, because it validates the name as non-empty and always passes
`Some(name)`. The user therefore retypes a name the app would have derived,
and a blank name is an error message rather than a sensible default.

## What Changes

- Choosing a folder while the name field is blank SHALL fill the name with
  that folder's last path component. The value lands in the field, visible and
  editable, rather than being applied silently at submit time.
- A name the user has already typed is never overwritten. The fill happens
  only when the field is empty, so picking a different folder to correct a
  mistake does not discard a deliberate name.
- The folder section moves from last to second, between the identity section
  (name, avatar) and the agent section (type, shell command, persona,
  activation). Choosing a folder then reads top-to-bottom: the folder is
  picked, the name fills in, and the rest of the form follows.

Not in scope:

- Any change to what makes a form submittable. The folder is still required
  and still must be an existing directory; the name is still required. This
  changes when the name gets a value, not whether one is needed.
- The store's own fallback in `AgentStore::create`. It stays as the last line
  of defence for callers that pass no name at all - the MCP `agent_create`
  tool among them - and is not removed just because the dialog stops relying
  on it.
- The editor's other sections, their contents, or their order relative to each
  other.

## Capabilities

### New Capabilities

None. `agent-editor-ui` already owns what the dialog offers and what its
fields default to.

### Modified Capabilities

- `agent-editor-ui`: gains two requirements. One fixes the order the dialog
  presents its sections in, so the folder is asked for before the fields that
  describe the agent rather than after them. The other says that choosing a
  folder names a still-unnamed agent after it, and that a name the user
  supplied is never replaced.

## Impact

- `crates/knot/src/agent_editor/pickers.rs` - `choose_folder` currently sets
  `folder_path` and notifies. It gains the conditional name fill. The fill
  needs the window to set an `InputState` value, which this path does not
  currently take; the async block runs with only an app context, so how the
  write reaches the input is the one real design question here.
- `crates/knot/src/agent_editor/render.rs` - the `Render` impl composes four
  sections in a fixed order; `folder_rows` moves to second. The scroll
  container's comment explains itself in terms of the folder row being last
  and is part of the change, not an afterthought.
- `crates/knot-agents/src/store/helpers.rs` - `last_path_component` already
  derives a name from a folder and is `pub(super)`. The editor is in another
  crate, so either it widens or the rule is expressed where the editor can
  reach it. Do not write a second copy of the same three lines.
- No new dependency. No change to any persisted shape. One new user-facing
  string only if the fill needs explaining, which it should not - the filled
  field explains itself.
