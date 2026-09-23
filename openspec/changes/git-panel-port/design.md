# Design

## Context

See `proposal.md` - Why for motivation, and `specs/git-panel-ui/spec.md` for the
behavior contract. What shapes the approach here:

- **`knot-git` is complete but for one method.** `Repository` has the staging,
  commit, status and stats operations; `diff.rs` parses unified diffs. There is
  no method that *fetches* a diff — `consts::DIFF` and `DIFF_STAGED_FLAG` exist
  and `parse_diff` has no caller anywhere in the workspace.
- **`knot-watch` has no consumer.** `Watch::new` has no call site. Its
  `GIT_STATUS_DEBOUNCE` (1 s) and `RESUME_SETTLE` (500 ms) constants were
  written for this panel and cite the Swift `GitFileWatcher` by name.
- **The render path rule is load-bearing here.** `refresh_cache.rs` exists
  because `git diff --numstat` was put on the render path three times. The panel
  issues more git calls than any existing view, so it is the biggest test of
  that rule so far.
- **`WorkspaceWindow` owns the runtime and the repaint poll.** It has a tokio
  runtime (`sessions.rs` uses `runtime.spawn_blocking`) and a ~30 Hz
  `repaint_poll_tick` that calls `cx.notify()` only when a dirty predicate says
  something changed.

## Goals / Non-Goals

**Goals:**

- Every git call — read and write — off the render path, reusing
  `RefreshCache` rather than hand-rolling a second polling discipline.
- Make the Swift diff-load race unrepresentable rather than guarding against it.
- One module tree that stays inside the 700-line file limit by concern.

**Non-Goals:**

- Changing `knot-git`'s existing signatures, its error model, or the
  `git-operations` contract. The one addition is additive.
- A general-purpose diff viewer. The panel renders what `parse_diff` produces;
  no syntax highlighting, no word-level intra-line diff, no side-by-side view.
- Staging individual hunks or lines. `knot-git` is path-scoped and stays so.
- Persisting panel width or the list/diff split. Both are view state, per spec.

## Decisions

### Where the panel attaches: a second panel in the content column

`content_column` (`workspace_window/render/content.rs`) already picks one pane
for the selected agent from a priority chain — markdown file, mermaid diagram,
stopped placeholder, panel pane, terminal grid. The git panel becomes a sibling
*beside* that pane, in a horizontal resizable split, with per-agent open state on
`WorkspaceWindow` (`git_panel_open: BTreeSet<Uuid>`, mirroring the existing
`panel_input_expanded: BTreeSet<Uuid>`).

Alternatives rejected:

- **A `WorkspaceViewMode` variant.** `view_mode` is one value for the whole
  window and `is_takeover()` means "replace the content". The panel is scoped to
  one agent and must leave that agent's content visible, so it is per-agent state,
  not a window mode.
- **`gpui-component`'s `Sheet`.** It is a real sliding-overlay primitive
  (`Placement::Right`, resizable), but it slides from the *window* edge and would
  cover the sidebar as well. The Swift panel is a sibling in an `HStack`, so the
  content narrows rather than being occluded — a split is the faithful shape, and
  it also keeps the agent list reachable while reviewing.

### Two caches, keyed so the diff race cannot happen

```
type GitStatusCache = RefreshCache<Uuid, GitStatusSnapshot>;
type GitDiffCache   = RefreshCache<DiffKey, DiffOutcome>;

struct DiffKey { agent: Uuid, path: PathBuf, staged: bool }
enum GitStatusSnapshot { NotARepository, Failed(String), Loaded(RepoStatus) }
enum DiffOutcome        { Failed(String), Loaded(Box<FileDiff>), Absent }
```

Both value types are `Clone + PartialEq`, which `RefreshCache` requires;
`RepoStatus` and `FileDiff` already derive both.

The key choice is what removes the Swift race. Swift keeps one `selectedDiff`
slot, so a late reply for a deselected file overwrites the current one. Keying
the cache by `(agent, path, staged)` means a late reply writes to *its own*
entry, and the render only ever reads the entry for the current selection — the
stale diff has nowhere to land. The spec's "the first row's diff is never shown"
falls out of the data structure instead of needing a generation counter.

`GitStatusSnapshot::Failed` and `DiffOutcome::Failed` carry a `String` rather
than `GitError`, because `GitError` is neither `Clone` nor `PartialEq` and the
cache needs both. The message is formatted at the boundary where the error is
caught.

Errors are a *value* in the cache, not an absent entry. `refresh_cache.rs`
already warns about this: an absent entry means "no answer yet", so encoding a
failure as absence leaves the panel on "loading…" forever.

### Reads: claim-refresh; writes: explicit invalidation

Reads copy `sessions.rs::refresh_diff_stats` exactly — `claim_refresh` during
render, `runtime.spawn_blocking`, `writer.record(...)`, and a `take_changed()`
wired into `repaint_poll_tick`'s dirty predicates.

Writes are not on a cadence, so they do not use `claim_refresh`. A click handler
spawns one blocking task that, in order: pauses the watch, runs the git
operation, `forget`s the agent's status entry, `forget`s that agent's diff
entries, `forget`s the agent's `diff_stats` entry, and schedules the watch
resume. Forgetting rather than writing means the next frame re-claims and the
normal read path produces the new state — there is one way state arrives, not
two.

`diff_stats` invalidation is what makes the dashboard card follow a commit
instead of showing pre-commit counts until `DIFF_STATS_MAX_AGE` elapses.

### The watch: one per open panel, dirty-flag callback

One `knot_watch::Watch` per open panel, held in
`git_panel_watches: BTreeMap<Uuid, Arc<Watch>>` on `WorkspaceWindow`, started
when the panel opens and stopped when it closes or the agent goes away.

The relevance predicate is the spec's work-tree filter, and it lives in
`knot-watch`'s caller, not in `knot-watch` — `Watch` takes its predicate from
the caller by design, and `knot-discovery`'s filter is precedent for the filter
living with the thing that knows what is relevant.

`Watch`'s callback runs on a tokio task with no GPUI context, so it cannot touch
the cache or call `cx.notify()`. It sets an `Arc<AtomicBool>`; `repaint_poll_tick`
reads and clears the flag, forgets the status entry, and notifies. This is the
same "flip a flag, let the poll act on it" shape `RefreshCache`'s own `dirty`
flag uses.

`Watch::start` calls `tokio::spawn`, so it must run inside the runtime — the
call site needs `let _guard = self.runtime.enter();` exactly as
`refresh_diff_stats` does. Starting it outside the runtime panics at the
`tokio::spawn`, and it panics on the main thread at panel-open time.

### Pause/resume bracketing

The spec requires the panel's own writes not to trigger it. `Watch` already has
`pause()`/`resume()` with a 500 ms post-resume settle. The bracket must be
`pause → operation → refresh completes → resume`, not `pause → operation →
resume`: git's writes are still landing when the process exits, and the status
read that follows touches `.git` itself. Resume is scheduled after the follow-up
status read records, so the settle window covers the tail of both.

### The new `knot-git` method

```rust
pub fn file_diff(&self, path: &str, staged: bool) -> Result<Option<FileDiff>>
```

argv is `["diff", "--no-color"]`, plus `"--staged"` when staged, plus `"--"` and
the path. The `--` separator is new (`consts::PATHSPEC_SEP`) and is not optional:
without it a path that matches a ref name is ambiguous to git, and agent branches
and file paths collide often enough for that to be real.

Returns `Option` rather than `Vec` because the call is path-scoped to one file,
so `parse_diff` yields at most one entry. Swift takes `.first` of a vector and
silently drops any remainder; returning `Option` says the same thing without the
silent drop.

### Diff rendering: virtualized lines, spans per line

Lines render through `gpui_kit::list` with a `ListState`, the mechanism
`panel_view/render.rs` already uses with a 400 px overdraw. That satisfies the
spec's "work per frame does not grow with the number of diff lines" without
capping what the user can read.

Within a line, the `terminal_view.rs` pattern applies: an `h_flex` per line, a
`div` per styled run, `text_color` / `bg` per run. The two line-number gutters
are fixed-width leading runs. `whitespace_nowrap` plus a horizontally scrolling
container gives the spec's no-wrap requirement.

Colors reuse the existing palette: `consts.rs` already documents `COLOR_IDLE` as
"green: an agent that is idle, and a diff's added lines" and `COLOR_ERROR` as
red for removed lines. No new diff colors are needed; a hunk-header color is the
one addition.

A hard line cap remains as a backstop for a pathological diff — virtualization
bounds *drawing*, but `parse_diff` still materializes every line into memory
first, and a multi-hundred-megabyte generated file should not be held in the
cache. The cap is where the spec's truncation notice comes from.

### Module layout

`crates/knot/src/git_panel/`, `mod.rs` declaring only, per the repo rule:

| File | Holds |
| --- | --- |
| `state.rs` | `DiffKey`, `GitStatusSnapshot`, `DiffOutcome`, the cache aliases, selection |
| `sections.rs` | Grouping a `RepoStatus` into the four sections, row model |
| `status_list.rs` | Rendering the branch header, sections and rows |
| `diff_pane.rs` | Diff header, line virtualization and per-line spans |
| `actions.rs` | Stage/unstage/discard/bulk dispatch, invalidation, confirmation |
| `commit_window.rs` | The commit prompt window |
| `watch.rs` | Relevance predicate, watch lifecycle, dirty flag |
| `tests.rs` | Grouping, selection invalidation, relevance predicate |

Splitting `sections.rs` from `status_list.rs` is deliberate: grouping and
selection invalidation are pure functions over a `RepoStatus` and are the part
worth unit-testing, so they must not be tangled with element construction.

## Risks / Trade-offs

- **A per-frame `claim_refresh` for the selected diff key is a map lookup on a
  `PathBuf` key.** → Cheap, but unlike `diff_stats`'s `Uuid` it allocates on
  clone. Only the selected key is claimed per frame, so it is one lookup, not
  one per row.
- **Diff entries accumulate as the user clicks through files.** → `forget` on
  panel close and `retain` scoped to the open agent, the same lifecycle
  discipline `refresh_cache.rs` documents for keys that no longer exist.
- **`Watch` holds a `notify` recursive watcher per open panel.** → Bounded by
  open panels, and stopped on close. A large monorepo working tree is the cost
  case; the relevance predicate limits the *callbacks*, not the watch itself.
- **Confirming discard is friction on an action Swift made instant.** → It
  guards the one irreversible operation in the panel. Staging stays instant.
- **The commit prompt as a separate window diverges visibly from the Swift
  app.** → It follows this codebase's stated convention instead, which the
  broadcast sheet already set. Consistency within the port beats fidelity to a
  macOS sheet.
- **A very large `RepoStatus` (tens of thousands of untracked files) makes the
  list itself the cost.** → Not addressed here; the sections render as plain
  children because realistic working trees are tens of rows. If it bites, the
  same `gpui_kit::list` mechanism the diff pane uses applies to the list.

### Entry point: the agent header's diff stats

The agent header already renders the agent's diff stats
(`workspace_window/creation.rs::render_diff_stats` → `app_state::diff_stats_row`).
That row is the existing "this folder has N changes" indicator, so it becomes the
control that opens the panel showing what those changes are — the question the
row already raises.

This keeps `agent-list-ui` unmodified. Routing the entry point through the agent
row context menu would need a delta to that capability for one menu item, and
would bury the panel behind a right-click when there is already a visible
affordance pointing at exactly the thing the panel shows.
