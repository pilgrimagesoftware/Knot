# Design

## Context

See proposal.md - Why. The constraint that shapes everything here is that
one function serves four callers:

```rust
pub(crate) fn find_config_option<'a>(options: &'a [knot_acp::ConfigOption],
                                     categories: &[&str])
                                     -> Option<&'a knot_acp::ConfigOption>
```

`crates/knot/src/workspace_window/creation.rs:47`. The permission-mode,
model and effort selectors call it from
`crates/knot/src/workspace_window/panel/input.rs:354`, `:365`, `:373`, and
the prompt's risk colour from
`crates/knot/src/workspace_window/panel/pane.rs:334`. Fixing the function
fixes all four; there is no per-caller variation to preserve.

The candidate lists are already mixed in kind. `["mode",
"permission_mode", "permission-mode"]` and `["model"]` name ACP
categories. The effort slot's list holds both: `"thought_level"` and
`"thought-level"` are ACP categories, while `"effort"`, `"reasoning"`,
`"reasoning_effort"` and `"reasoning-effort"` are id-shaped spellings that
no category will ever carry. So the callers have been passing two
vocabularies to a parameter named `categories` from the start, and half of
one list has been inert. That is why widening the match is a correction
rather than a loosening.

`knot_acp::ConfigOption` needs no change: `category` is already
`Option<String>` (`crates/knot-acp/src/protocol/mod.rs:87`), and `id` and
`name` are always present.

## Goals / Non-Goals

**Goals:**

- A compliant agent that omits `category` gets the same UI as one that
  supplies it.
- An agent that supplies a matching `category` keeps its current
  behaviour exactly - no reordering, no different option chosen.
- The reason a slot is empty becomes "the agent declared nothing that
  matches", never "the agent declared it but we did not look there".

**Non-Goals:**

- Reworking how risk level is inferred. `permission_risk_level`
  (`crates/knot/src/panel_view/style.rs:102`) keeps its keyword match.
  One keyword was added during verification: the live roster (task 4.2)
  showed Codex's `agent-full-access` rating `Neutral` although it is
  Codex's counterpart to `bypassPermissions`, so "full access" now maps
  to the highest risk level (task 4.6).
- Rendering non-`select` option types. ACP tells clients to ignore option
  types they do not recognize, and the pickers cannot draw them.
- Reporting a missing option to the user as an error. The empty state
  stays the empty state; it just stops being reached for the wrong
  reason.

## Decisions

### Match in passes: category, then id, then name

Category first, so an agent that labels its options keeps today's exact
behaviour and the well-labelled case cannot be stolen by a coincidental id
match elsewhere in the list. Only when no option in the list carries a
matching category does the id pass run, then the name pass.

Alternative considered: one pass testing category-or-id-or-name per
option. Rejected - it lets an option that merely happens to be *named*
"mode" outrank a later option properly categorized `mode`, which inverts
the protocol's own priority.

### Match on exact, case-insensitive equality - never substring

The current code already uses `eq_ignore_ascii_case`, and it has to stay
that way once ids are in scope: `"mode"` is a substring of `"model"`, so a
substring match would let the permission-mode slot claim the model option.
The candidate sets are disjoint under exact equality and overlapping under
any looser rule.

### Break ties by the agent's array order

ACP: clients "SHOULD use the ordering of the `configOptions` array as
provided by the Agent as the primary way to establish priority and resolve
ties"
([RFD](https://agentclientprotocol.com/rfds/session-config-options)).
`Iterator::find` already returns the first match in array order, so each
pass gets this for free - it needs stating, not building.

### Keep the `kind == "select"` filter in every pass

A non-select option cannot populate a picker whichever field matched it.
The filter stays outermost so the fallback passes cannot smuggle one in.

### Leave every candidate list alone

An earlier revision of this design proposed adding `thought_level` to the
effort slot, on the belief that it had no ACP category to match. It
already has one, and `thought-level` besides. The lists need no edit: the
three passes are what the id-shaped spellings in that list were always
waiting for.

### Leave the parameter named `categories`? No - rename it

Once the list is matched against ids and names too, `categories` is a lie
about what the argument means, and the effort slot proves the name was
already misleading. Rename to `candidates`, with a doc comment naming the
three passes. This is a private associated function; the rename is
mechanical.

## Risks / Trade-offs

- **An agent names an unrelated option exactly `mode` and gets picked up
  by the permission slot.** → Requires exact equality against a
  three-word candidate list, in an agent that also declared no properly
  categorized mode option. The resulting selector would be populated with
  the wrong option's values, which is visible and reversible, against a
  feature that is currently invisible for every uncategorized agent.
  Accepted.
- **Widening the match changes behaviour for an agent that is currently
  working.** → It cannot: the category pass runs first and returns before
  any fallback. A test pins this - an agent with both a categorized
  option and a differently-identified one must still resolve to the
  categorized one.
- **The tests keep passing while live agents stay broken, as they did
  before.** → The fixture that hid the gap
  (`crates/knot-acp/src/client/tests.rs:333`) hard-codes
  `"category":"mode"`. A second fixture with no category at all goes in
  beside it, so the uncategorized path is exercised rather than assumed.
- **The spec rescope for the selector keybinding touches
  `openspec/specs/permission-prompt-ui/spec.md`, which PR #387 also
  edits.** → #387 modifies the adjacent "Permission prompt is
  keyboard-operable" block, not this one. If #387 lands first, rebase
  before merging; the hunks are neighbours, so a conflict would be
  textual and trivial.

## Open Questions

- Which categories the ACP agents actually in use advertise today is still
  unmeasured - the diagnosis establishes that Knot's requirement is
  non-compliant regardless, so the fix does not wait on the answer. The
  manual verification task records what the live roster turns out to be,
  which tells us whether the id/name fallback is load-bearing in practice
  or only insurance.
