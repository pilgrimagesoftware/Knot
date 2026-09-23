# Design

## Context

See `proposal.md` — Why. The constraints below come from reading
`gpui-base` 0.6.4 (the vendored version) and 0.6.6 (the latest release);
both behave identically on every point that matters here.

**The composer today.** `crates/knot/src/workspace_window/panel/prompt.rs`
builds one `TextareaState` per agent with
`.placeholder(..).submit_on_enter(..).auto_grow(1, max_rows)`, and
`set_panel_input_expanded` re-issues `set_auto_grow(1, max_rows)` to switch
between `PANEL_INPUT_ROWS_COLLAPSED` and `PANEL_INPUT_ROWS_EXPANDED`.
`input.rs` renders it inside the chip strip, the queued-prompt list and the
control bar.

**Where styled ranges live.** `gpui-base` renders styled ranges from two
seams, and both are reachable only from `EditorState`:

- `InputHighlighter`, installed with `set_highlighter_factory`. It is stored
  *inside* the `LayoutMode::CodeEditor` variant, so it exists only while the
  layout mode is `CodeEditor`.
- `TextDecorationCollection`, from `create_decorations_collection`. It is
  stored in `state.extras`, which is keyed off the **mode marker**, not the
  layout mode: `InputModeKind::Extras` is `()` for both `InputMode` and
  `TextareaMode` — and `impl InputExtras for ()` takes the default
  `decoration_layers()`, which returns an empty `Vec`. A textarea therefore
  cannot carry a decoration, ever.

**Decorations do not require code-editor layout.** `element.rs`'s
`highlight_lines` matches on `state.mode`; its `_ =>` arm — every non-
`CodeEditor` layout — still composes `state.extras.decoration_layers()`
(`gpui-base-0.6.4/src/input/base/element.rs:1516`). So an `EditorState` in
*any* layout mode paints its decorations.

**The blocker.** `auto_grow`, `set_auto_grow`, `rows` and `set_rows` all sit
in `impl InputBaseState<TextareaMode>`
(`gpui-base-0.6.4/src/input/base/state.rs:9158`). `EditorState` has no row
sizing at all. Meanwhile the element's auto-grow machinery is entirely
generic over the mode marker: `element.rs` reads `state.mode.is_auto_grow()`,
`.rows()` and `.max_rows()` from `impl<M: InputModeKind>` code, and
`LayoutMode::update_auto_grow` sizes from `display_map.wrap_row_count()`
without consulting the marker. **Nothing in the layout or render path
requires a textarea to auto-grow — only the `impl` block's bound does.**

So the composer needs `EditorState` (for decorations) and `auto_grow` (for
its existing sizing), and today it cannot have both.

## Goals / Non-Goals

**Goals:**

- One styling pass that serves all four treatments — tokens, markdown,
  attachment chips — so they cannot disagree about where a range starts.
- Preserve every composer behaviour the panel already depends on: send key,
  lookup keys, expand/collapse, focus-on-selection, drafts, paste and drop.
- Keep the render path free of parsing work proportional to buffer size.
- Keep the upstream fork small enough to re-verify mechanically on a bump,
  and delete it as soon as upstream releases.

**Non-Goals:**

- See `proposal.md` — Non-goals, which this does not repeat.
- Reaching `LayoutMode::CodeEditor`. This design deliberately never enters
  it; see "Decision: stay in `AutoGrow`".
- Making the styling extensible by third parties. The scanner knows four
  construct families and is not a plugin point.

## Decisions

### Decision: relax the bound upstream, patch in the interim

Move `auto_grow`, `set_auto_grow`, `rows` and `set_rows` from
`impl InputBaseState<TextareaMode>` to `impl<M: MultiLineMode>
InputBaseState<M>`. This is a bound relaxation with no body change: `rows`
and `set_rows` already match all three `LayoutMode` variants, and
`auto_grow`/`set_auto_grow` just assign `LayoutMode::auto_grow(..)`.

Open the PR at `longbridge/gpui-kit`; until it releases, consume a fork
through `[patch.crates-io]` on `gpui-base` in the root `Cargo.toml`.

*Why not the alternatives.* Reimplementing auto-grow in Knot means
reproducing `display_map.wrap_row_count()`, which is crate-private, so our
height would be an independent guess at the element's own wrapping and would
drift on any change to it. Dropping to two fixed heights is a user-visible
regression in a control the user touches constantly. Phasing the styling
behind an upstream release puts a shipped feature on a third party's
schedule.

*What it costs.* A fork of a crate that `gpui-component` and `gpui-kit` both
depend on, so the patch applies workspace-wide and must be rebased on every
gpui-kit bump. The mitigation is that the diff is four `impl` headers: a bump
that cannot be rebased mechanically is a signal the upstream shape changed,
which is exactly when a human should look. `docs/adr/` records the fork and
its exit condition, and removing it is a task in `tasks.md`, not a
someday-item.

### Decision: decorations, not `InputHighlighter`

Compute the styling ourselves and publish it through
`create_decorations_collection`, rather than implementing `InputHighlighter`.

`InputHighlighter` lives inside the `CodeEditor` layout variant, so using it
would force the composer into code-editor layout — which is the one thing
`panel-rich-input`'s "The composer stays a composer" requirement forbids.
Decorations are marker-keyed instead of layout-keyed, so they work in
`AutoGrow`. The trait is also shaped for a tree-sitter-style incremental
parser with fold ranges, none of which a prompt box wants.

### Decision: stay in `AutoGrow`, never `CodeEditor`

`EditorState::new` starts in `CodeEditor` layout; the first thing the
composer does is `.auto_grow(1, max_rows)`, which replaces the mode outright.

This is what makes "the composer stays a composer" true *by construction*
rather than by turning off flags. Line numbers, the gutter, indent guides,
folding, auto-closing brackets and smart indent are all fields of the
`CodeEditor` variant or gated on `is_code_editor()`. A composer that is never
in that variant cannot grow that chrome, and a future gpui-kit that adds a
new code-editor affordance cannot leak it into the composer either.

The corollary: `set_line_number`, `set_auto_close`, `set_smart_indent` and
`set_folding` are no-ops for us — they match on `LayoutMode::CodeEditor` and
fall through. Calling them would be cargo-culting; the code should not.

### Decision: separate layers per concern

Create three decoration collections, not one. Collections compose in creation
order with the first winning on a conflicting property, which gives a
deliberate precedence:

1. **Attachment chips** — narrow, exact, and must not be overridden.
2. **Tokens** (`/`, `@`) — bounded runs.
3. **Markdown** — the broadest, and the one most likely to overlap the others
   (a token inside a fenced block, say).

Separate collections also let each be recomputed on its own schedule: an
attachment changes on attach and detach, tokens and markdown on every edit.
One merged collection would mean rebuilding all three to move one chip.

### Decision: one scanner, incremental over the edited range

A single scanner produces all the ranges, from the buffer text plus the
attachment table. It is a plain function over `&str` with no GPUI types in
its signature, so it is unit-testable without a window.

The composer re-renders per keystroke — `.claude/rules/rust-structure.md`
records three defects from I/O and work on the render path, and
`crates/knot/src/diff_stats.rs` exists because of them. So the scanner
rescans the **dirty region**, not the buffer: an edit's line, widened
outward to the enclosing block construct, and widened again while the edit
can have opened or closed a fence. Only fences and block quotes cross lines;
emphasis, code spans and tokens do not, which bounds the widening.

`InputEvent::Change` carries the edit, and `create_decorations_collection`'s
ranges already slide with edits (`adjust_annotations`), so untouched ranges
stay correct without being recomputed.

### Decision: `@` is a third `LookupSource`

`crates/knot/src/panel_commands/mod.rs` already defines
`trait LookupSource` with the comment "A third source later is a new
implementor, not a change to the popup." This is that source.

Two things do change beyond adding an implementor:

- `LookupEntry::matches` is a case-insensitive substring test with "no fuzzy
  ranking, deliberately" — right for a few dozen commands, wrong for
  thousands of files. Matching and ranking become properties of the source:
  commands keep substring matching, files get subsequence matching with a
  score. The popup renders whatever order the source returns.
- `active_token` in `panel_commands/token.rs` recognises one trigger. It
  becomes trigger-aware, returning which trigger is under the caret, and the
  popup dispatches on that. This is what makes the two lookups mutually
  exclusive by construction rather than by two flags that can both be set.

### Decision: enumeration is watch-driven, reusing `knot-discovery`

File enumeration runs off the render path in a background task, once per
agent, cached. `knot-discovery` already owns debounced `notify`-backed folder
watching; the mention source reuses that rather than adding a second watcher
or polling.

Git-tracked enumeration goes through `knot-git` (tracked plus
untracked-but-not-ignored) so `.gitignore` is honoured without
reimplementing ignore rules. The non-repository fallback is a filesystem walk
with the Swift reference's exclusion set.

### Decision: the attachment chip is text, with the table as the truth

An attachment is a token in the buffer plus a row in the pending-context
table. The table is authoritative for what gets sent; the token is how the
user sees and moves it.

The two are reconciled after every edit, in one direction: **the buffer
decides**. A token the user deleted detaches its row. This makes
`acp-panel-ui`'s two "removing either removes both" scenarios fall out of one
rule instead of two-way syncing, which would have to arbitrate a conflict it
can never actually resolve.

Removing a row from the strip deletes its token as a programmatic edit, which
then flows through the same reconciliation.

### Decision: theme-resolved styles, rebuilt on appearance change

Treatments are built from `cx.theme()` at decoration-build time, not stored
as constants. GPUI re-renders on appearance change; the styling rebuild hangs
off the same notify, so `panel-rich-input`'s appearance-switch scenario needs
no separate observer.

Per that spec's legibility requirement, no treatment distinguishes itself by
hue alone — each also carries weight, slant or a background, so the styling
survives a low-contrast theme and does not rely on colour vision.

## Risks / Trade-offs

- **The fork is a standing tax on gpui-kit bumps.** → Four `impl` headers,
  no body change; a bump that will not rebase mechanically is a signal, not
  a chore. Recorded in an ADR with its exit condition, and its removal is a
  task, not a someday-item. `markdown_view.rs` already establishes the
  project's habit of writing down what to re-check on a bump.

- **`EditorState` may differ from `TextareaState` in ways not read here** —
  IME composition, context menu, selection, accessibility. → The migration
  task lands with the behaviour-preservation scenarios from
  `panel-rich-input`'s "The composer stays a composer" as tests, before any
  styling is added, so a regression is attributed to the swap rather than
  found later under the styling.

- **The scanner is a second markdown implementation** next to
  `markdown_view.rs`. → They do different jobs (styled ranges over a
  half-typed buffer vs. rendered elements from a finished document) and the
  proposal records that. The risk is the two disagreeing about what counts
  as emphasis. Accepted: a composer that styles slightly more eagerly than
  the renderer is a far smaller harm than a per-keystroke block parse.

- **Incremental scanning is where the subtle bugs will be** — a fence opened
  far above the edit, a paste spanning constructs, undo. → The scanner is a
  pure function, so the dirty-region result is checked against a
  scan-the-whole-buffer result in tests over the edit sequences from
  `panel-rich-input`'s "survives every way the composer changes". They must
  agree; a disagreement is the bug.

- **`input.rs` is 517 lines against a hard 700-line cap.** → It is split by
  concern before the feature work starts, not squeezed afterwards. The cap is
  not negotiable (`.claude/rules/rust-structure.md`).

- **Enumerating a large repository costs memory and time.** → Capped, as the
  Swift reference caps it, and `panel-file-mentions` requires the cap to be
  reported in the popup rather than silently truncating.

- **Chips are text, not embedded thumbnails.** → A toolkit limitation, not a
  preference; written into `panel-rich-input` so a later reader does not
  "fix" it by reaching for an API that does not exist.

## Migration Plan

1. Split `input.rs` by concern, no behaviour change. Green build gate.
2. Fork `gpui-base`, relax the four bounds, wire `[patch.crates-io]`, open
   the upstream PR, write the ADR.
3. Swap `TextareaState` → `EditorState` with `.auto_grow(..)`, no styling.
   The composer must be indistinguishable from before; that is the test.
4. Add the scanner and the three decoration layers.
5. Add the `@` source, generalise `active_token`, per-source matching.
6. Add attachment tokens and buffer-decides reconciliation.

Steps 3 through 6 are each independently revertable. Step 3 is the one with
schedule risk: if `EditorState` turns out to differ from `TextareaState` in a
way that cannot be reconciled, it surfaces there, before any styling work is
spent on top of it.

**Rollback.** No persisted state changes shape, so rollback is reverting
code. Drafts are plain text before and after. The `[patch.crates-io]` entry
is removed with step 2.

## Resolved Questions

Both were settled on 2026-09-23, against the styled composer in a debug
build, which is the condition each was deferred for.

**Spaces in a mention: escaping, not quoting.** `@a\ b/c.rs` rather than
`@"a b/c.rs"`. Escaping is one rule in the scanner - a token ends at the
first *unescaped* whitespace - where quoting needs the scanner to track an
opening quote and decide what an unclosed one means, which is a second
unclosed-construct problem next to markdown's. Both satisfy
`panel-file-mentions`, and neither reads well in a prompt; the escaped form
at least degrades to something a reader recognises. `token_end` and
`escape_token` sit beside each other in `composer_scan::tokens` and a
round-trip test holds them to being one rule read in two directions.

**An attachment chip shows the file's base name.** `@screenshot.png`, not
the path relative to the agent's folder. A chip is read at a glance, the
strip above the input already carries the same name with its thumbnail, and
the absolute path still reaches the agent on the `Attached:` line
`send_panel_prompt` appends - so nothing is lost by the shorter form.

The cost is that two attachments whose file names match produce the same
chip text. That is a reconciliation problem rather than a presentation one,
and it is handled where it arises: `surviving_attachments` counts
occurrences rather than looking for the reference, so with two identical
chips and one deleted exactly one row survives. A relative path would have
made the collision rarer without removing it - two files of the same name in
two checkouts of the same repository collide either way - so the counting
was needed regardless, and the base name is the better-reading of two forms
that need the same machinery.
