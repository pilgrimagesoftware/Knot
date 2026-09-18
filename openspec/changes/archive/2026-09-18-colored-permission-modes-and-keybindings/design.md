## Context

See proposal.md - Why. Relevant existing shape:

- `ConfigOption` / `ConfigOptionValue` (`crates/knot-acp/src/protocol.rs`)
  carry a `value: String` id (e.g. `"bypassPermissions"`, `"plan"`) and a
  human `name`, but no risk metadata - agents declare arbitrary values.
- `PermissionRequest` (same file) carries only `rpc_id`, `tool_call_id`,
  and allow/deny `options` - **no mode information**. The inline prompt's
  color can only reflect the panel's current permission-mode config option
  (`PanelState::config_options`), not anything on the request itself.
- `render_panel_config_selector` (`main.rs`) is generic across mode/model/
  effort selectors; it must stay generic - only the permission-mode call
  site should apply risk coloring, not the model/effort ones.
- `render_permission_prompt` (`panel_view.rs`) is a free function, not a
  method, and doesn't currently receive `config_options`.
- App-wide keybindings already exist via `actions!(knot_app, ...)` +
  `cx.bind_keys([KeyBinding::new(...)])` in `main.rs` (e.g. `cmd-,` for
  `OpenSettings`). No panel-scoped action/keybinding exists yet.

## Goals / Non-Goals

**Goals:**
- Derive a risk level from a mode's string id via keyword matching, with no
  new dependency and no per-agent configuration.
- Reuse gpui-kit's existing color/theme primitives (`rgb(...)`, `cx.theme()`)
  rather than introducing a new color system.
- Scope new keybindings to the panel (active only while a panel/prompt has
  relevant focus), not global app shortcuts.

**Non-Goals:**
- No per-agent or user-configurable risk-keyword list in this change - the
  keyword set is a fixed constant.
- No change to the ACP protocol or `knot-acp` wire types to carry explicit
  risk metadata - keyword matching on the existing `value`/`name` strings is
  sufficient and avoids a protocol change no agent implements today.
- No keybinding customization UI - bindings are fixed defaults, consistent
  with the one existing app-level binding.

## Decisions

**Risk classification: fixed keyword matching, three levels.**
A small function `permission_risk_level(mode_value: &str) -> RiskLevel`
(`RiskLevel::{Danger, Safe, Neutral}`) does case-insensitive substring
matching against two fixed keyword lists (danger: `bypass`, `yolo`,
`danger`; safe: `plan`, `read`), returning `Neutral` otherwise. Alternative
considered: an explicit mapping table keyed by known mode ids (e.g.
`"bypassPermissions"`) - rejected because it silently fails to classify any
mode id an agent adapter hasn't been special-cased for; substring matching
degrades gracefully across adapters that all converge on the same English
words for the same concepts.

Colors: reuse the existing `ERROR_COLOR`/danger red already in
`panel_view.rs` for `Danger`, a new muted green constant for `Safe`, and
`cx.theme()`'s default button styling (no override) for `Neutral`.

**Where the mode lookup happens.** `main.rs` already resolves the
permission-mode `ConfigOption` via `find_config_option` before calling
`render_panel_config_selector`; add the risk-level computation at that same
call site and pass a `RiskLevel` into a new coloring parameter, rather than
having the generic selector function special-case "permission" by string
comparison on `element_id`. This keeps `render_panel_config_selector`
reusable for model/effort without a hidden behavior branch.

For the inline prompt, `render_panel_pane` already holds `config_options`
before it currently drops the state lock; thread the resolved `RiskLevel`
(computed the same way) into `panel_view::render_panel` /
`render_permission_prompt` as a new parameter, defaulting to `Neutral` when
no permission-mode option is declared.

**Keybindings: two new panel-scoped actions.** Add
`PanelPermissionAllow` / `PanelPermissionDeny` and
`PanelOpenPermissionSelector` to the `actions!(knot_app, ...)` block,
bound via `cx.bind_keys` in the same window-setup path as the existing
`cmd-,` binding. Handlers look up the focused/relevant panel's session
state the same way the existing `on_decision`/click handlers do (via
`panel_sessions` + `pending_permission`), so a fired action with no pending
request is a no-op, matching the "no pending prompt ignores the keybinding"
scenario. Exact key combinations (e.g. `cmd-shift-a` / `cmd-shift-d`) are
chosen at implementation time to avoid colliding with existing terminal
passthrough or textarea shortcuts - verified against `crates/knot/src/main.rs`'s
current `bind_keys` calls and the terminal's key-forwarding path before
finalizing.

## Risks / Trade-offs

- [Keyword matching misclassifies an unusual agent-specific mode id] ->
  Mitigation: unmatched ids fall back to `Neutral` (today's unstyled
  appearance), never silently mislabel a mode as falsely "safe"; only
  explicit danger keywords escalate to the warning color.
- [A panel-scoped keybinding collides with a key the embedded terminal
  (Ghostty) or textarea already forwards] -> Mitigation: verify against
  existing bindings and terminal passthrough during implementation before
  picking final key combinations; keep mouse controls fully functional as a
  fallback regardless.
- [Coloring the prompt from "currently active mode" rather than the mode
  in effect at request time could show a stale color if the user switches
  modes while a request is pending] -> Mitigation: acceptable given ACP
  provides no per-request mode data; documented in the spec as reflecting
  the active mode, not a per-request one.

## Migration Plan

Additive UI change with no data migration. Ship behind no flag - risk
coloring and keybindings apply immediately on update. Rollback is a plain
revert if a chosen key combination turns out to collide with something in
practice.
