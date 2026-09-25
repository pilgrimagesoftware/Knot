## 1. Widen the lookup

- [x] 1.1 Rename `find_config_option`'s `categories` parameter to
      `candidates` and document the three passes it now runs
      (`crates/knot/src/workspace_window/creation.rs:47`); update the four
      call sites and the three existing tests. Verify with `make build`.
      The call sites pass the argument positionally, so the rename reached
      none of them.
- [x] 1.2 Implement the passes: category, then `id`, then `name`, each
      filtered to `kind == "select"` and each matched with
      `eq_ignore_ascii_case`, returning the first match in the agent's own
      array order. A later pass runs only when the earlier one found
      nothing across the whole list, so a categorized option cannot be
      outranked by a coincidental id or name match.
- [x] 1.3 Nothing to do - the premise was wrong. `"thought_level"` and
      `"thought-level"` were already in the effort selector's candidate
      list (`crates/knot/src/workspace_window/panel/input.rs:380-381`);
      the proposal read a truncated view of it and reported the slot as
      unable to match on category at all. It always could. The effort slot
      carries the same single defect as the other two - category required -
      and 1.2 fixes it there too. No candidate list changes.

## 2. Pin the behaviour

- [x] 2.1 Extend `crates/knot/src/tests/workspace_window_config.rs`: an
      option with no category matches on `id`; one with an unrecognized
      category matches on `id`; one matching only on `name` is found; a
      non-select option is ignored on every pass; and `"mode"` does not
      match an option identified `"model"`, which would break the slots
      apart from each other.
- [x] 2.2 Assert the precedence directly: given a list holding both a
      properly categorized option and an earlier one whose `id` also
      matches, the categorized one wins. This is the test that proves the
      change is additive for agents that work today.
- [x] 2.3 Assert the tie-break: given two options that both match on the
      same pass, the one the agent listed first wins. Asserted on both the
      category pass and the id pass, the latter with two *different*
      candidates so the winner is the agent's first entry rather than the
      first candidate in our own list.
- [x] 2.4 Add a fixture agent to `crates/knot-acp/src/client/tests.rs`
      that declares its config options with no `category` at all, beside
      the existing one at `:333` that hard-codes `"category":"mode"`, so
      the uncategorized path is exercised end to end rather than only at
      the lookup. The test also pins that a missing category arrives as
      `None` rather than acquiring a default that could match by accident.
- [x] 2.5 Verify the new tests fail without the fix rather than passing
      for the wrong reason: with the two `or_else` fallback passes deleted,
      exactly the four fallback tests fail and the six guard tests still
      pass. Restored afterwards.

## 3. Correct the spec wording

- [x] 3.1 Verified the rescope against the implementation: the handler
      sits on the workspace root element
      (`crates/knot/src/workspace_window/render/mod.rs:424`), its binding
      carries no key context (`app_bootstrap.rs:374`), and the only gate
      is the selected agent being Panel-mode. Input-area focus is not
      required, so the delta's wording is right.

      Nothing is written to `openspec/specs/` here. This repo syncs a
      change's delta at archive time, not during implementation - the
      previous change shipped in #350 and had its delta applied only when
      #387 archived it. The rescope rides the same path.

## 4. Verification

- [x] 4.1 Run `make` and confirm fmt, size-check, clippy, test and build
      all pass. Exit 0, 1521 tests, no failures.
- [x] 4.2 Manual, against a live ACP agent - the check this change exists
      to satisfy, and the one no unit test can stand in for. Start a
      Claude session and a Codex session, and record what each actually
      advertises: the `id`, `name` and `category` of every entry in
      `configOptions`. That roster is the evidence issue #194 asked for,
      and it decides whether the id/name fallback is load-bearing or
      insurance.

      Captured 2026-09-25 from each adapter's `session/new` response,
      driven directly over stdio (`initialize`, then `session/new`), in
      the agent's own array order.

      `@agentclientprotocol/claude-agent-acp` 0.81.2:

      | `id`     | `name`    | `category`      | values                                                 |
      |----------|-----------|-----------------|--------------------------------------------------------|
      | `mode`   | Mode      | `mode`          | default, acceptEdits, plan, auto, bypassPermissions    |
      | `model`  | Model     | `model`         | default, opus[1m], claude-fable-5-1[1m], sonnet, haiku |
      | `effort` | Effort    | `thought_level` | default, low, medium, high, xhigh, max                 |
      | `fast`   | Fast mode | `model_config`  | on, off                                                |

      `@agentclientprotocol/codex-acp` 1.13.1:

      | `id`                 | `name`             | `category`           | values                                           |
      |----------------------|--------------------|----------------------|--------------------------------------------------|
      | `mode`               | Mode               | `mode`               | read-only, agent, agent-full-access              |
      | `collaboration_mode` | Collaboration mode | `collaboration_mode` | default, plan                                    |
      | `model`              | Model              | `model`              | gpt-6-luna, gpt-5.6-terra, gpt-5.6-luna, gpt-5.5 |
      | `reasoning_effort`   | Reasoning effort   | `thought_level`      | low, medium, high, xhigh, max                    |

      Every option is `select` and carries a category, so all three slots
      in both agents resolve on the category pass: for these two adapters
      the id/name fallback is insurance, not load-bearing. If either shows
      an empty selector in the running app, this lookup is not the cause.
      `fast` and `collaboration_mode` match no slot, as intended; exact
      matching keeps `collaboration_mode` out of the permission slot.

      Found in passing: `permission_risk_level`
      (`crates/knot/src/panel_view/style.rs:102`) rated Codex's
      `agent-full-access` as `Neutral`, because only "bypass", "yolo" and
      "danger" mapped to `Danger`. Fixed under 4.6.
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
- [x] 4.6 Rate Codex's `agent-full-access` as `Danger`, confirmed neutral
      in the running app during 4.3. `permission_risk_level` folds `-` and
      `_` to spaces and matches "full access", so the id and the display
      name ("Full access") both qualify; `read-only` stays `Safe` and
      `agent` stays `Neutral`. Pinned by
      `codex_permission_modes_are_classified` in
      `crates/knot/src/panel_view/tests.rs`, with a scenario added to the
      `permission-prompt-ui` delta.
