# Tasks

## 1. The registry

- [ ] 1.1 Add a `panel_commands` module with a documented in-code built-in
      command list (token + description, seeded from
      `plugin/claude/commands/*.md`) and unit tests asserting every entry has
      a slash-free token and a non-empty description.
- [ ] 1.2 Add a skill registry provider that scans the selected agent's
      configured skill roots for `SKILL.md` frontmatter (name + description).
      Unit tests: a root with two skills yields two entries; a missing root
      yields none without error.
- [ ] 1.3 Define the shared entry type and the registry interface both sources
      implement. Verify a test that wires both sources behind the interface
      and filters by token prefix.

## 2. Token detection

- [ ] 2.1 Implement slash-token detection from the prompt input's value and
      caret (leading `/` on the caret's line, filter text up to end of token).
      Unit tests: `/` alone, `/p`, token in the middle of prose (no popup),
      and `/p mid-edit` (token ends at whitespace).
- [ ] 2.2 Prototype surgical token replacement against gpui-kit's
      `TextareaState` API (replace span, caret after token). If the API cannot
      express it, implement the whole-buffer/append fallback from the design
      and record the deviation in this change.

## 3. The popup

- [ ] 3.1 Render the lookup popup above the panel prompt row, listing filtered
      registry entries (token + description), with the empty-match state
      dismissed. Verify in the app that typing `/` opens it and a matchless
      filter closes it.
- [ ] 3.2 Wire Up/Down selection, Enter/Tab insert, and Esc dismissal through
      the input's key-capture path, plus dismissal on blur and on the token
      being edited away. Verify each in the app.
- [ ] 3.3 Confirm inserted text flows through the normal send path unchanged;
      verify in the app that a completed prompt is delivered as ordinary text
      and nothing executes.

## 4. Polishing

- [ ] 4.1 Memoize the skill scan per focused agent, refreshed on agent
      switch, and verify the lookup opens without a stall on the second and
      later opens in one session.
- [ ] 4.2 Route lookup-facing strings (empty state, tooltips) through
      `knot_core::l10n::t` and verify via a string/locale check.

## 5. Verification

- [ ] 5.1 `make rust` passes clean (fmt, clippy, tests, build).
- [ ] 5.2 Exercise the full flow in the app: open the lookup, filter to one
      entry, insert, send, and confirm the agent receives the completed token
      as a prompt.