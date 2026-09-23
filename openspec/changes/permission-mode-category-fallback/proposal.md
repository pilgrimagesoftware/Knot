# Proposal

## Why

`WorkspaceWindow::find_config_option` only matches a Session Config Option
whose `category` is present and matches one of the caller's candidates
(`crates/knot/src/workspace_window/creation.rs:47-58`). ACP says that field
is optional: "Categories are for UX purposes only and MUST NOT be required
for correctness. Clients MUST handle missing or unknown categories
gracefully"
([Session Config Options](https://agentclientprotocol.com/protocol/v2/session-config-options)).

So an agent that is fully compliant but omits `category` is invisible to
the lookup. Every caller folds the resulting `None` into its own negative
case, and nothing reports an error:

- the permission-mode, model and effort selectors each render their
  "this agent doesn't report ..." empty state
  (`crates/knot/src/workspace_window/panel/input.rs:354`, `:365`, `:373`)
- the inline permission prompt keeps its neutral border, because the risk
  level resolves to `RiskLevel::Neutral`
  (`crates/knot/src/workspace_window/panel/pane.rs:334`)

That is the whole of issue #194's remaining half. The unit tests pass
because the fixture hard-codes `"category":"mode"`
(`crates/knot-acp/src/client/tests.rs:333`), so the gap only appears
against a real agent.

Correction, found while implementing: this proposal originally claimed the
effort selector was wrong a second way - searching only id-shaped
spellings, and so unable to match on category at all. That was read off a
truncated view of the candidate list. `"thought_level"` and
`"thought-level"` are already in it
(`crates/knot/src/workspace_window/panel/input.rs:380-381`), beside the
id-shaped spellings. The effort slot carries the same single defect as the
other two, and no candidate list needs changing.

## What Changes

- Make the lookup ACP-compliant: when no option carries a matching
  `category`, fall back to matching the option's `id`, then its `name`,
  against the same candidates. A missing or unknown category stops being
  fatal, which is what the protocol requires of a client.
- Resolve ties by the agent's own ordering. ACP says clients "SHOULD use
  the ordering of the `configOptions` array as provided by the Agent as
  the primary way to establish priority", so the first match in array
  order wins rather than an arbitrary one.
- Keep the `kind == "select"` filter. A non-select option cannot drive a
  picker, and ACP tells clients to ignore option types they do not
  recognize.
- Correct the permission-mode selector requirement's scope wording to
  describe what ships: the keybinding is registered on the pane and gated
  on the selected agent being Panel-mode, not on the input area holding
  focus.
- **No behavior change for an agent that already supplies a matching
  category.** The category pass runs first and wins.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `acp-panel-ui`: the three input-area selector requirements gain the
  obligation to find the agent's option without requiring `category`, so a
  compliant agent that omits it still gets a populated selector rather
  than the empty state.
- `permission-prompt-ui`: the two risk-colouring requirements gain the
  same obligation, since both read the same lookup; and the
  permission-mode selector's keyboard requirement is rescoped to match the
  implementation.

## Impact

- `crates/knot/src/workspace_window/creation.rs`: `find_config_option`
  gains the fallback passes. It is the single choke point all four callers
  share, so the fix lands in one function.
- `crates/knot/src/workspace_window/panel/input.rs`: untouched. The three
  candidate lists are already right; only the matcher was wrong.
- `crates/knot/src/tests/workspace_window_config.rs`: the three existing
  tests keep asserting the category pass; new ones cover a missing
  category, an unknown category, id and name fallback, array-order ties,
  and a non-select option still being ignored.
- `crates/knot-acp/src/client/tests.rs`: a fixture agent that declares its
  options with no `category`, so the gap cannot reopen unnoticed.
- No new dependencies, and no change to `knot_acp::ConfigOption`, whose
  `category` is already `Option<String>`.
- Carried in from the archived `permission-prompt-keybinding-hints`
  change: its task 3.3, the manual check that the prompt's decision
  buttons read "Allow ⇧⌘A" / "Deny ⇧⌘D" and that key and click both
  resolve a request. Verifying this change needs a live permission prompt
  anyway, so both checks ride the same occasion.
