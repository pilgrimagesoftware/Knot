# Tasks

## 1. Backend: the missing diff method

- [x] 1.1 Add `consts::PATHSPEC_SEP` (`--`) to `knot-git` and verify `cargo build -p knot-git` succeeds
- [x] 1.2 Add `Repository::file_diff(path, orig_path, staged) -> Result<Option<FileDiff>>` building `git diff --no-color [--staged] -- <path>` and parsing with `parse_diff`; verify a new test in `crates/knot-git/tests/` gets a hunk for a modified tracked file
- [x] 1.3 Verify `file_diff` returns `None` for a path with no changes, and the staged variant returns the index-against-HEAD diff for a path that is both staged and further modified — the case the panel's two rows depend on
- [x] 1.4 Verify `file_diff` on a path whose name also matches a branch name resolves as a path, proving the `--` separator works, and that a binary file yields a `FileDiff` with `binary` set
- [x] 1.5 Take `orig_path` so a rename stays a rename: git needs both sides to detect one, and the destination alone reports a whole new file. Verify `tests/file_diff.rs` pins both the unscoped failure and the two-path fix

## 2. Panel state and caches

- [x] 2.1 Create the `crates/knot/src/git_panel/` module tree with `mod.rs` declaring only (no implementation) and verify `make size-check` and `cargo build -p knot` pass
- [x] 2.2 Add `knot-watch` to `crates/knot/Cargo.toml` as a path dep, matching how every sibling crate is declared there, and verify the workspace builds
- [x] 2.3 Define `DiffKey`, `GitStatusSnapshot`, `DiffOutcome` and the two `RefreshCache` aliases in `git_panel/state.rs`; verify they satisfy the `Clone + PartialEq` bounds by instantiating both caches in a compiling test
- [x] 2.4 Implement grouping a `RepoStatus` into the four sections in `git_panel/sections.rs`; verify unit tests in `git_panel/tests.rs` cover a staged-and-modified path appearing in both the staged and unstaged sections, and empty sections being omitted
- [x] 2.5 Implement selection as a `(path, staged)` pair with invalidation against a new `RepoStatus`; verify a test that selection survives a refresh keeping the pair, and is cleared when the path keeps only its other side

## 3. Reads off the render path

- [x] 3.1 Implement the status read: `claim_refresh` + `runtime.spawn_blocking` + `writer.record`, copying `sessions.rs::refresh_diff_stats`; verify the panel populates on open in a debug build
- [x] 3.2 Map a failed `Repository::status()` to `GitStatusSnapshot::Failed` and a non-repository folder to `NotARepository`; verify the panel shows an error rather than a clean tree for a folder whose `.git` has been made unreadable
- [x] 3.3 Implement the diff read keyed by `DiffKey`, claimed only for the current selection; verify selecting a file shows its diff and that selecting a second file before the first arrives never shows the first
- [x] 3.4 Wire both caches' `take_changed()` into `repaint_poll_tick`'s dirty predicates and verify the panel redraws when a refresh lands without any unconditional `cx.notify()`
- [ ] 3.5 Verify no git command runs from a render: type continuously into the commit message field with a status and diff loaded and confirm no `git` process is spawned

## 4. Working tree list

- [x] 4.1 Render the branch header with ahead and behind counts shown only when non-zero, and nothing when HEAD is detached; verify against a repository in each of the three states
- [x] 4.2 Render the four sections with titles and entry counts, omitting empty ones, in one scroll region; verify against a working tree exercising all four
- [x] 4.3 Render file rows with each row's own side's change type, file name, and directory when present; verify a staged-added-and-unstaged-modified path shows different glyphs on its two rows
- [x] 4.4 Implement row selection and its visible marking following the `render/sidebar.rs` selected-row pattern; verify the two rows of a staged-and-modified path select independently

## 5. Diff pane

- [x] 5.1 Render the diff header with the path and addition/deletion counts, each shown only when non-zero; verify against an additions-only diff
- [x] 5.2 Render diff lines through `gpui_kit::list` with per-run spans, two fixed-width line-number gutters, and per-classification colors reusing `COLOR_IDLE` / `COLOR_ERROR`; verify additions, deletions, context and hunk headers are each distinguishable
- [x] 5.3 Verify line numbers appear on the correct side per classification: new only for additions, old only for deletions, both for context, neither for hunk headers
- [x] 5.4 Implement no-wrap with horizontal scrolling and verify a line wider than the pane stays on one line with its gutters aligned
- [x] 5.5 Render the binary, no-hunks and no-selection states; verify each against a binary file, a rename with no content change, and an empty selection
- [x] 5.6 Implement the line cap with a stated truncation notice and verify a generated file far over the cap renders promptly and says it was truncated
- [x] 5.7 Implement the user-adjustable split between the list and the diff pane and verify both panes stay usable at the extremes of the drag

## 6. Mutations

- [x] 6.1 Implement stage, unstage and discard for one path off the render path, each followed by forgetting the agent's status, diff and `diff_stats` entries; verify the list reflects each action without a manual refresh
- [x] 6.2 Gate each row's actions by its side — stage when not staged, unstage when staged, discard when neither staged nor untracked; verify an untracked row offers only stage
- [x] 6.3 Implement stage-all and unstage-all on the unstaged and staged sections only; verify stage-all also stages untracked files and that untracked and conflicts sections offer no bulk control
- [x] 6.4 Add the discard confirmation naming the path, using the `confirm_then` / `open_alert_dialog` pattern with a danger-variant confirm button; verify cancelling leaves the working tree untouched and that staging is not confirmed
- [x] 6.5 Surface a failed git operation without applying it to the displayed tree; verify by discarding a path made read-only

## 7. Commit

- [x] 7.1 Show the commit control only while something is staged; verify it appears and disappears as the last staged path is staged and unstaged
- [x] 7.2 Implement the commit prompt as a window following `broadcast_sheet.rs` — textarea state, explicit focus, Escape to dismiss, boxed result callback; verify it opens focused and Escape closes it
- [x] 7.3 Disable confirm while the trimmed message is empty and while a commit is in flight; verify a whitespace-only message cannot be committed
- [x] 7.4 Trim the message's surrounding whitespace while preserving internal blank lines; verify the committed message's subject and body survive via `git log`
- [x] 7.5 Dismiss and refresh on success; on failure keep the window, show the error and retain the message — verify by committing with a pre-commit hook that rejects

## 8. Watching

- [x] 8.1 Implement the work-tree relevance predicate in `git_panel/watch.rs`; verify unit tests cover `.git/index`, `.git/HEAD`, `.git/refs/*` as relevant, `.git/objects` and `.git/COMMIT_EDITMSG` as not, dotfiles as not, and `.gitignore` as relevant
- [x] 8.2 Start one `Watch` per open panel inside the tokio runtime (`runtime.enter()` before `start()`) and stop it on close; verify opening a panel does not panic and that closing it leaves no running watch
- [x] 8.3 Route the watch callback through an `AtomicBool` read by `repaint_poll_tick`, which forgets the status entry and notifies; verify an external edit to a tracked file refreshes the panel
- [x] 8.4 Bracket every panel git operation with `pause` before and a `resume` scheduled after the follow-up status read records; verify staging a file does not produce a second refresh from its own writes
- [ ] 8.5 Verify a burst of fifty file changes collapses to one refresh

## 9. Integration and wiring

- [x] 9.1 Add `git_panel_open: BTreeSet<Uuid>` and `git_panel_watches` to `WorkspaceWindow` and render the panel as a resizable split beside the agent's pane in `content_column`; verify the agent's content stays visible with the panel open
- [x] 9.2 Make the agent header's diff stats row open and close the panel for that agent; verify it toggles and that the close control does the same
- [x] 9.3 Implement the panel's default width of 500 clamped to 350–800 on drag, not persisted; verify the clamp at both ends and that a reopened panel is 500 again
- [x] 9.4 Forget an agent's cache entries and stop its watch when its panel closes or the agent is removed; verify no entries survive for a removed agent
- [ ] 9.5 Verify the dashboard card's diff stats update immediately after the panel commits, rather than after `DIFF_STATS_MAX_AGE`

## 10. Conventions and close-out

- [x] 10.1 Add every user-facing string to `crates/knot-core/locales/en.yml` under `git_panel.*`, using `t_with` for any sentence embedding a value; verify with key-resolution tests, touching `knot-core` first so the build is not stale
- [x] 10.2 Move any new panel constants into `crates/knot/src/consts.rs` with a comment saying what each decides; verify no layout number was moved there and no decision left inline
- [x] 10.3 Run `make` and verify `fmt-check`, `size-check`, `lint`, `test` and `build` all pass
- [x] 10.4 Verify no crate-wide `allow` was added, `mod.rs` files declare only, and no colocated test module pushes a file over 700 lines
- [ ] 10.5 Run the app against a repository with staged, unstaged, untracked and conflicted files and walk the full loop — review a diff, stage, discard, commit — confirming each spec scenario in `specs/git-panel-ui/spec.md`
