# Proposal

## Why

An agent's answer often carries the thing the user actually wants — a command,
a patch, a config snippet — inside a fenced code block, and the only way to
get it out today is a drag-select across a virtualized list or a "copy
response" that brings the whole prose along with it. Every other agent UI puts
a copy button on the code block itself; Knot's Markdown surfaces do not.

## What Changes

- Every fenced code block on a Markdown surface gains a copy button in its
  upper-right corner. Activating it places that block's code — the fenced
  content, without the fences or the language tag — on the system clipboard
  and confirms with the same notification style the panel's existing copy
  actions use.
- The button is added once, in the shared `markdown_view` constructor, so both
  Markdown surfaces (the panel's assistant messages and the `display-markdown`
  pane) get it and cannot drift apart.
- Two new localization keys for the button's tooltip and its confirmation.

Non-goals:

- No copy affordance on inline code spans. A code block is a deliberate,
  delimited unit; an inline span is a word inside a sentence and has no corner
  to hang a button off.
- No syntax highlighting, no language label, no "wrap/unwrap" control. This
  change adds one affordance to the block chrome and nothing else.
- No change to the existing per-message copy actions ("copy prompt", "copy
  response"). They stay as they are.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `acp-panel-ui`: the Markdown rendering requirements gain a requirement that
  a rendered fenced code block carries a copy control in its upper-right
  corner, on both Markdown surfaces, copying the block's code alone.

## Impact

- `crates/knot/src/markdown_view.rs` — the shared constructor gains a
  `code_block_actions` callback. This is upstream gpui-base's own hook, which
  already positions its element absolutely at the block's top-right, so no
  layout work is needed and the heading-renderer style of claiming a node is
  not repeated here.
- `crates/knot-core/locales/en.yml` — `panel.copy_code`, `panel.copied_code`.
- `crates/knot/src/tests/markdown_view.rs` — coverage for the copied text and
  the localization keys.
- Both consumers of `markdown_view` inherit the change without edits:
  `crates/knot/src/panel_view/message.rs` and
  `crates/knot/src/workspace_window/panel/pane.rs`.
- No new dependency. `gpui-kit` 0.6.4 already exposes everything used.
