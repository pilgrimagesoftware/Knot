# Tasks

## 1. The wire field

- [ ] 1.1 Add `tool_call_title: Option<String>` to `PermissionRequest` and
      parse it from `toolCall.title` in the client, leaving it `None` when
      absent. Update/extend `knot-acp` tests: a request with `toolCall.title`
      surfaces the title, one without surfaces `None`, and the existing
      flat-`toolCallId` fallback still parses.
- [ ] 1.2 Confirm existing `PermissionRequest` construction sites and finders
      compile with the additive field (it is `Option`, so nothing should
      break); run `cargo test -p knot-acp`.

## 2. Name resolution

- [ ] 2.1 Add a pure `display_name` resolution on `PanelState` (request
      title → known card's title → card's kind → raw id) with unit tests for
      all four rungs, including a request whose card arrived after it and one
      with no card at all.
- [ ] 2.2 Wire `render_permission_prompt` to format the resolved name into
      the existing sentence and verify in the app that a tool call titled
      "Reading configuration file" shows that title in the prompt.

## 3. Verification

- [ ] 3.1 `make rust` passes clean (fmt, clippy, tests, build).
- [ ] 3.2 Exercise in the app: a permission request for a visible tool call
      shows its title; one that arrives before any card renders shows a
      title when the adapter provides it and the raw id otherwise; allow and
      deny both still resolve exactly as before.