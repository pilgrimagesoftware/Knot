# Tasks

## 1. knot-acp

- [x] 1.1 `protocol/permission.rs`: `PermissionOptionKind` (Display/FromStr), lenient `kind` parsing, `PermissionDecision::{AllowAlways, Choose}`, resolution by kind with the kindless fallback
- [x] 1.2 `permission_result` answers through the resolution; unit tests on Claude's and exit-plan-mode's real option lists, an unknown kind, the kindless list
- [x] 1.3 End-to-end: a fake agent sending Claude's options receives `allow`, `allow_always` and `reject` for Allow, Always Allow and Deny

## 2. knot

- [x] 2.1 `PanelPermissionAllowAlways` action, `cmd-alt-shift-a`, handler; the selected-agent path answers only a decision the request offers
- [x] 2.2 `prompt_choices`: one choice per option with its label, emphasis and key; kindless requests keep Allow/Deny; tests
- [x] 2.3 Keybinding and hint tests for Always Allow

## 3. Verification

- [x] 3.1 `make` green
