# Tasks

## 1. Search field border

- [x] 1.1 In `crates/knot/src/workspace_window/render/pull_requests_toolbar.rs`, pass the theme's `border` color to `search_field` and set it with `.border_color(..)` on the `Input`, with a comment on why only this field overrides it; verify `make` passes
- [x] 1.2 Launch Knot on macOS in light and in dark appearance, open the Pull Requests view with at least one recorded pull request, and confirm the unfocused search field shows a visible border, the focused one shows the accent focus border, and a Settings text field looks as it did before. If the border is too faint in light appearance, apply the opaque-composite fallback from design.md and repeat; record the result in this task. Verified by the user in a running build on 2026-10-07: the border is visible in light and dark appearance, so the fallback was not needed.
