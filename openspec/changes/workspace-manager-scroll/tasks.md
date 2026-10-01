# Tasks

## 1. Setup

- [ ] 1.1 Create the dedicated worktree + feature branch for this change per AGENTS.md (`<checkout>-Worktrees/workspace-manager-scroll`, branched from `develop`) and verify `git branch --show-current` reports the feature branch, not `develop`/`main`

## 2. Implementation

- [ ] 2.1 In `crates/knot/src/workspace_manager/render.rs`, wrap the workspace rows in a scroll container: replace `.child(v_flex().gap_2().children(rows))` with a `div().id("workspace-list").flex_1().overflow_y_scroll()` whose child is that same `v_flex`, leaving the toolbar `h_flex` and error banner as pinned siblings, and verify `cargo build -p knot` compiles (add `.min_h(px(0.))` to the container if the short-window test overflows instead of scrolling)
- [ ] 2.2 Add a real-window render test (following the `tests/workspace_dialog.rs` harness: temp settings root, `VisualTestContext`, seeded `WorkspaceManager`) that seeds more workspaces than fit a short window, scrolls the list, and asserts the bottom-most workspace row becomes reachable / the scroll offset changes, and verify `cargo test -p knot` passes it
- [ ] 2.3 Confirm the few-workspaces case is unchanged: with fewer rows than fit, all rows are visible without scrolling, and verify this is asserted (or visibly holds) in the same test

## 3. Verification

- [ ] 3.1 Run the full local gate `make fmt`, `make lint`, `make test` in the worktree and verify all pass, with no `.rs` file over the 700-line limit (`make size-check`)