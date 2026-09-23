## 1. Widen the lookup

- [ ] 1.1 Rename `find_config_option`'s `categories` parameter to
      `candidates` and document the three passes it now runs
      (`crates/knot/src/workspace_window/creation.rs:47`); update the four
      call sites and the three existing tests. Verify with `make build`.
- [ ] 1.2 Implement the passes: category, then `id`, then `name`, each
      filtered to `kind == "select"` and each matched with
      `eq_ignore_ascii_case`, returning the first match in the agent's own
      array order. A later pass runs only when the earlier one found
      nothing across the whole list, so a categorized option cannot be
      outranked by a coincidental id or name match.
- [ ] 1.3 Add `"thought_level"` to the effort selector's candidate list
      (`crates/knot/src/workspace_window/panel/input.rs:373`), keeping the
      existing id-shaped spellings.

## 2. Pin the behaviour

- [ ] 2.1 Extend `crates/knot/src/tests/workspace_window_config.rs`: an
      option with no category matches on `id`; one with an unrecognized
      category matches on `id`; one matching only on `name` is found; a
      non-select option is ignored on every pass; and `"mode"` does not
      match an option identified `"model"`, which would break the slots
      apart from each other.
- [ ] 2.2 Assert the precedence directly: given a list holding both a
      properly categorized option and an earlier one whose `id` also
      matches, the categorized one wins. This is the test that proves the
      change is additive for agents that work today.
- [ ] 2.3 Assert the tie-break: given two options that both match on the
      same pass, the one the agent listed first wins.
- [ ] 2.4 Add a fixture agent to `crates/knot-acp/src/client/tests.rs`
      that declares its config options with no `category` at all, beside
      the existing one at `:333` that hard-codes `"category":"mode"`, so
      the uncategorized path is exercised end to end rather than only at
      the lookup.

## 3. Correct the spec wording

- [ ] 3.1 Apply the `permission-prompt-ui` delta's rescope of
      "Permission-mode selector is keyboard-operable" to match what
      ships: gated on the selected agent being Panel-mode, not on the
      input area holding focus. If PR #387 has landed, rebase onto
      `develop` first - it edits the adjacent requirement block.

## 4. Verification

- [ ] 4.1 Run `make` and confirm fmt, size-check, clippy, test and build
      all pass.
- [ ] 4.2 Manual, against a live ACP agent - the check this change exists
      to satisfy, and the one no unit test can stand in for. Start a
      Claude session and a Codex session, and record what each actually
      advertises: the `id`, `name` and `category` of every entry in
      `configOptions`. That roster is the evidence issue #194 asked for,
      and it decides whether the id/name fallback is load-bearing or
      insurance.
- [ ] 4.3 Manual, same session: confirm the permission-mode, model and
      effort selectors are populated rather than showing their empty
      states, and that the inline permission prompt's border takes the
      risk colour of the active mode instead of staying neutral.
- [ ] 4.4 Manual, same session - carried in from the archived
      `permission-prompt-keybinding-hints` change, task 3.3, which was
      never done: confirm the prompt's decision buttons read
      "Allow ⇧⌘A" / "Deny ⇧⌘D" legibly against the primary button fill,
      that pressing each key resolves the request, and that clicking
      still works.
- [ ] 4.5 Close issue #194 only once 4.2-4.4 are recorded. Its "Done
      when" asks for the feature to be established as reachable, not
      merely for the code to be changed.
