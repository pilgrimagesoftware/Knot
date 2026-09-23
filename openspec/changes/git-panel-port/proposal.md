# Proposal

## Why

Knot can show that an agent's repository has changes but not what they are. The
Swift app answers that with a sliding panel over the agent's folder: the working
tree as a grouped list, a unified diff for the selected file, per-path stage /
unstage / discard, and a commit sheet — the loop that lets someone review an
agent's work without leaving the app. The Rust port has none of it, so reviewing
what an agent did means leaving for a terminal or another client.

The backend for it already exists and is unused. `knot-git` implements and tests
`stage`, `unstage`, `discard`, `stage_all`, `unstage_all` and `commit`, and
parses unified diffs into `FileDiff` / `Hunk` / `DiffLine`; `knot-watch`
implements the debounced pause/resume watch, down to a `GIT_STATUS_DEBOUNCE`
constant that cites the Swift `GitFileWatcher` by name. None of it has a caller:
`knot_git::parse_diff` has no consumer anywhere in the workspace, the staging and
commit operations are reached only from `knot-git`'s own tests, and
`knot_watch::Watch::new` has no call site at all. This change is the missing
consumer, not new plumbing.

It is also one of the subsystems the port never spec'd, which is why it was
missed. The spec is therefore part of the change, not a formality.

## What Changes

- **New `git-panel-ui` capability**: a per-agent sliding panel over the agent's
  folder, with a drag handle, the working tree grouped into staged / unstaged /
  untracked / conflicted sections, per-row and bulk staging actions, a
  selectable file whose unified diff renders in a lower pane, and a commit
  sheet.
- **`knot-watch` gets its first consumer**: the panel composes `Watch` with a
  git-specific relevance predicate rather than growing a second watcher. The
  predicate itself is a spec change (see Modified Capabilities) because today's
  `file-watching` spec only describes the source-folder filter.
- **One missing backend method**: `Repository` gains a path-scoped diff
  (`git diff --no-color [--staged] -- <path>`, parsed by the existing
  `parse_diff`). The `git-operations` contract already requires exactly this
  shape, so this is an implementation gap, not a contract change.
- **The agent's diff stats refresh when the panel changes the tree**, so a stage
  or commit is reflected on the dashboard card without waiting out the normal
  `DIFF_STATS_MAX_AGE` cadence.

### Behavior deliberately not carried over

Recording these is the point of writing the spec: a dropped behavior that is
never written down becomes the next gap of this kind.

- **Main-thread git mutations.** Swift runs `stage` / `unstage` / `discard`
  synchronously on the main actor while only status and diff reads are detached
  (`GitPanelViewModel.swift`). Under GPUI that stalls the render loop, and the
  repo has an explicit rule against I/O on the render path with a cache built
  because the rule was broken three times. **Dropped**: every git call in the
  panel runs off the render path.
- **Two stacked debounce layers.** Swift gets FSEvents' own 1.0 s coalescing
  latency *plus* a 1.0 s application debounce, so a change takes at least two
  seconds to appear. **Dropped**: one debounce, the existing
  `knot_watch::consts::GIT_STATUS_DEBOUNCE` (1 s).
- **Out-of-order diff loads.** Swift fires an uncancelled detached task per
  selection, so two quick clicks race and the slower one wins.
  **Dropped**: the panel shows the diff for the current selection or nothing.
- **Selection invalidation that ignores the staged flag.** Swift clears the
  selection only when the path disappears from the status, so a file that
  changes which side it sits on keeps a stale diff on screen. **Dropped**:
  selection is invalidated on the `(path, staged)` pair it was made with.
- **A failed `git status` that looks like a clean tree.** Swift's
  `GitRepository.status()` swallows the error and returns an empty status, so a
  broken repository renders as "Working tree clean". **Dropped**: `knot-git`
  already returns a `Result`, and the panel surfaces the error.
- **Unbounded diff rendering.** Swift renders every line of every hunk with no
  cap. GPUI re-renders per frame, so a large diff built as plain children is
  paid for every frame. **Dropped**: the panel bounds what it builds.
- **Amend, and any commit flag beyond `-m`.** Not in the Swift panel, not in
  scope here. Stated so the absence is a decision rather than an oversight.
- **Discarding an untracked file.** Swift offers discard only for tracked,
  unstaged rows, because `git restore` does not remove untracked files. Carried
  over as-is; there is deliberately no "delete untracked file" action.

### Behavior added beyond the Swift reference

- **Discard is confirmed.** Swift discards on a single click of a hover-revealed
  icon. Discard destroys uncommitted work irreversibly, and this panel exists to
  review work an agent did that the user has not read yet — the worst place in
  the app for an unguarded misclick. The port asks first. Staging and unstaging
  stay unconfirmed; each is undone by its opposite.
- **The commit prompt is a window.** `broadcast_sheet.rs` records the project's
  convention in its own doc comment: every dialog in this app that takes more
  than a yes/no is a window, not an in-place overlay. The commit prompt is a
  text area plus confirm/cancel — the same shape as the broadcast sheet — so it
  follows the same rule, rather than reproducing Swift's modal sheet.

## Capabilities

### New Capabilities

- `git-panel-ui`: the git panel — how it opens and is sized, the grouped working
  tree and its branch header, per-row and bulk stage/unstage/discard, file
  selection and the unified diff pane, the commit sheet and its validation, the
  panel's refresh triggers, and how it keeps git work off the render path.

### Modified Capabilities

- `file-watching`: add the work-tree relevance filter. The existing relevance
  requirement describes only the source-folder watch; the git panel's watch needs
  its own rule — which `.git` internals count (`index`, `HEAD`, anything under
  `refs/`) and which are noise, and that dotfiles outside `.git` are ignored
  except `.gitignore`, which changes untracked classification.

`git-operations` is deliberately **not** modified. Its diff requirement already
specifies `git diff --no-color` "optionally `--staged` and/or a single path";
what is missing is a `Repository` method exposing it, which is implementation.

## Impact

- **New**: a `git_panel` module tree under `crates/knot/src/`, split by concern
  to stay within the 700-line file limit — panel state, the working-tree list,
  the diff pane, and the commit sheet are separate files.
- **`crates/knot-git/src/repository.rs`**: one new path-scoped diff method,
  using the existing `consts::DIFF` / `DIFF_STAGED_FLAG` and `parse_diff`.
  Additive; no existing signature changes.
- **`crates/knot`**: gains a `knot-watch` dependency; a new `RefreshCache` user
  alongside `diff_stats` and `pull_request_state`; new constants in
  `crates/knot/src/consts.rs`; new `en.yml` keys for every string the panel
  shows.
- **`crates/knot/src/workspace_window/`**: an attachment point for the panel and
  a way to open and close it per agent.
- **`crates/knot/src/diff_stats.rs`**: the panel invalidates an agent's entry
  after a mutation so the dashboard card follows.
- **Unblocked**: `knot-watch` and the unused half of `knot-git` gain their first
  production consumers.
