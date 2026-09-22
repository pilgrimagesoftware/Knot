# Design

## Context

See proposal.md - Why.

Both Markdown surfaces are built by one constructor,
`crates/knot/src/markdown_view.rs::markdown_view`, which exists precisely so
the panel's assistant messages and the `display-markdown` pane cannot drift
apart. That module already carries one custom renderer — headings, claimed
through the block parser and block renderer hooks because gpui-kit exposes no
per-heading font family — and its module doc records the maintenance cost that
claiming a node imposes: the claimed branch no longer runs upstream, so
upstream's sizes, weights and padding are duplicated here and have to be
re-checked on every gpui-kit bump.

The relevant constraint for this change is that gpui-base (gpui-kit 0.6.4)
already has a first-class hook for exactly this affordance:
`TextView::code_block_actions`, a `Fn(&CodeBlock, &mut Window, &mut App) -> E`.
Upstream renders the returned element in a `div().id("actions").absolute()
.top_2().right_2()` over the code block's own background, and scopes element
ids per block so a plain id like `"copy"` cannot collide across blocks
(`gpui-base-0.6.4/src/text/node.rs`, `CodeBlock`'s render). `CodeBlock::code()`
returns the fenced content alone; `CodeBlock::lang()` the tag. The spec's
"upper-right corner, inside the block's background" is therefore upstream's
placement, not something this change has to build.

## Goals / Non-Goals

**Goals:**

- Add the affordance without adding a second claimed Markdown node, so
  `markdown_view`'s gpui-kit upgrade surface does not grow.
- Keep the copied text defined by the parser, not by string surgery on the
  Markdown source.

**Non-Goals:**

- Any change to how code blocks are drawn (highlighting, language label,
  wrapping). See proposal.md - Non-goals.
- Teaching the heading renderer anything. The two customizations stay
  independent.

## Decisions

### Use `code_block_actions`, not a claimed `MarkdownNode`

The heading renderer's pattern — claim the node in the block parser, draw it
here — is the wrong tool. It is what headings needed because there was no hook
for a heading's face; there *is* a hook for code-block chrome. Claiming the
code-block node would mean re-implementing the block's background, padding,
mono family, mono size, syntax-highlight plumbing and selection wiring, and
then re-checking all of it against upstream on every bump — the exact cost the
module doc warns about, paid for a button.

Alternative considered: claim the node, as headings do. Rejected for the
above. Alternative considered: wrap each `TextView` in our own container with a
single copy button for the whole surface. Rejected — it is the existing "copy
response" with extra steps, and gives nothing per block.

### The copied text is `CodeBlock::code()`

Not a re-scan of the Markdown source for fences. The parser already decided
where the block starts and ends, including for an indented block and for a
fence whose content contains backticks; re-deriving that would be a second,
disagreeing parser. This is also what makes the spec's "no fence markers, no
language tag" true by construction rather than by trimming.

### Always drawn, never hover-gated

Upstream paints the actions element unconditionally, and the panel's history
is a virtualized list whose items are recycled as they scroll — hover state
would have to live somewhere that survives recycling. A button that is simply
always there is both the cheaper implementation and the more reachable
control, so the spec requires it rather than merely permitting it.

Trade-off: the button sits over the block's first line, so a long first line
runs under it. Accepted; it is upstream's own layout, and the block's `p_3`
padding keeps the overlap to the line's tail rather than its start.

### Confirmation reuses the panel's notification pattern

`window.push_notification(Notification::info(...), cx)` with a localized
string, matching `panel_view/message.rs`'s copy-prompt and copy-response
handlers. The alternative — a transient in-button state ("Copied!") — needs
per-block state the renderer hook has nowhere to keep.

### Both strings go through `knot_core::l10n::t`

`panel.copy_code` and `panel.copied_code`, named after the existing
`panel.copy_response` / `panel.copied_response` pair. Per project convention
the tests assert the key resolves, never the English copy. The `panel.` prefix
is kept even though the Markdown pane is not the panel, because the existing
Markdown-surface strings already live under it and splitting the namespace for
one pair would be worse than the slight misnomer.

## Risks / Trade-offs

- **A gpui-kit bump changes the actions element's placement or drops the
  hook** → The spec pins the observable behavior (upper-right corner, inside
  the block), so a regression is a spec failure rather than a silent drift.
  Unlike the heading renderer, nothing here duplicates upstream values that
  could disagree quietly.
- **The button overlaps a long first line** → Accepted above.
- **A click inside the code block's area interfering with text selection** →
  The actions element is upstream's own hit target, already carved out of the
  block's selection handling; no new selection wiring is introduced.
- **An empty code block draws a button that copies nothing** → Harmless: the
  clipboard write succeeds with an empty string and the confirmation is
  honest. Not worth a special case, and suppressing the button would make the
  block's chrome inconsistent.
