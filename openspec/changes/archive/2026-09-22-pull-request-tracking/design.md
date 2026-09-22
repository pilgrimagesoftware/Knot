# Design

## Context

See `proposal.md` - Why. The relevant current state:

- There is no `gh` CLI use, no GitHub API client, and no HTTP client for a forge
  anywhere in the workspace. `knot-git` shells out to `git` only.
- Panel-mode agents produce `SessionUpdate::ToolCallStart` / `ToolCallUpdate` /
  `ToolCallResult` (`crates/knot-acp/src/protocol/mod.rs:163-212`), folded into
  `ToolCallCard { id, kind, title, status, content }` by
  `PanelState::apply_update` (`crates/knot/src/panel_state/mod.rs:240-310`).
  There is no typed command field - a command and its output ride inside
  `ToolCallContent::Text`.
- Terminal-mode agents have no ACP stream at all. Their output goes through
  `knot-terminal`'s `alacritty_terminal`-backed `Grid`
  (`crates/knot-terminal/src/grid/mod.rs`), which is a rendering surface; no
  code inspects its text today.
- `crates/knot/src/open_in.rs:120-126` already shells to `/usr/bin/open` for
  folders, with a `#[cfg(not(target_os = "macos"))]` no-op fallback.
- `crates/knot/src/diff_stats.rs` is the project's answer to "a subprocess whose
  result the UI wants": a claim-refresh cache with a TTL, a writer handed to
  whatever background mechanism the window has, and an `AtomicBool` the repaint
  poll reads. Its module docs say it has caught render-path I/O three times.
- The two windows differ in how they get off the main thread: `WorkspaceWindow`
  owns a tokio `Runtime` (`workspace_window/window.rs:76`) and uses
  `spawn_blocking`; `CommandCenterWindow` has none and uses `cx.spawn` plus the
  GPUI background executor.
- `knot-core`'s settings store holds five collection documents with atomic
  write-then-rename and tolerant per-record decode
  (`crates/knot-core/src/settings/store/documents.rs:22-99`).

## Goals / Non-Goals

**Goals:**

- One detection path that both agent kinds feed, so "a panel agent opened a PR"
  and "a terminal agent opened a PR" cannot drift apart.
- The forge dependency isolated behind one module, so its absence is a normal
  state rather than an error path threaded through the UI.
- Nothing new on the render path.

**Non-Goals:**

- Parsing what command produced the URL. The URL is the evidence; how it got
  printed is not interesting and not reliably available.
- A general link index. Only pull request URLs are recorded.
- Retrofitting the panel's tool-call rendering. Detection observes what is
  already flowing; it does not change what is shown.

## Decisions

### Detect by URL, not by command

Scan output text for the pull request URL pattern rather than trying to
recognize `gh pr create` or any other command.

- *Why*: the ACP stream has no typed command field, panel agents and terminal
  agents expose entirely different shapes, and the set of ways to open a pull
  request is open-ended - `gh`, `git push` printing GitHub's "create a pull
  request" hint, an MCP tool, a paste from the browser. All of them put the URL
  in the output; nothing else is common to all of them.
- *Consequence, accepted*: a pull request the agent merely *mentions* is
  recorded as one the agent opened. The spec's framing - "pull requests Knot has
  seen" - matches what is actually knowable, and the user can remove a row.
- *Alternative - a `report-pull-request` MCP tool in Knot's own catalog*:
  rejected as the primary mechanism. It only fires for agents that were told to
  call it, which is exactly the agents that need the feature least. It remains a
  clean additive path later; the record shape does not depend on the source.
- The pattern lives in one function, `scan_pull_request_urls(&str) ->
  impl Iterator<Item = PullRequestUrl>`, with `PullRequestUrl` holding host,
  owner, repo and number so normalization (`/files`, `#comment`) is a parse
  rather than a string trim. Pure, and the only part with real logic, so it is
  where the tests go.

### Two taps, one sink

Both detection taps call the same `record_pull_request(agent_id, url)` on the
store.

- **Panel**: in `PanelState::apply_update`, scan the text of
  `ToolCallContent::Text` as it is folded in - each chunk once, as it arrives.
  Already off the render path; `apply` runs on the ACP event drain.
- **Terminal**: scan the PTY byte stream as it is fed to the grid, not the
  rendered grid. Scanning the grid would mean re-reading a screen that changes
  every frame and would miss anything that scrolled past between polls; scanning
  the stream sees every byte exactly once. A small carry buffer spanning chunk
  boundaries handles a URL split across two PTY reads, sized to the longest URL
  the pattern can match.
- *Alternative - scan the rendered grid on a timer*: rejected for the above, and
  because it would put a scan on a path that runs per repaint.

### A new `knot-forge` crate wrapping `gh`

`knot-forge` exposes `probe() -> ForgeAvailability` (`Missing | Unauthenticated |
Ready`) and `pull_request_state(url) -> Result<PullRequestState, ForgeError>`,
running `gh pr view <url> --json number,title,state,isDraft,statusCheckRollup`
through the same subprocess-with-timeout shape `knot-git::Runner` uses.

- *Why a separate crate rather than a module in `knot-git`*: `knot-git` is about
  the local repository and has no network or credential concerns. A forge client
  has both, plus an optional external binary. Mixing them would make
  `knot-git`'s tests depend on `gh`.
- *Why `gh` rather than an HTTP client*: `gh` already holds the user's
  credentials, handles Enterprise hosts and token refresh, and adds no Rust
  dependency. An HTTP client would mean Knot storing a token, which is a
  materially larger change than this feature is worth.
- *Trade-off, accepted*: an external runtime dependency the user may not have.
  The spec makes its absence a first-class, non-failing state, and `probe()` is
  what the view's single message is derived from.
- `ForgeAvailability` is probed once per view opening rather than per pull
  request, so twenty rows do not mean twenty probes.

### State is fetched, never persisted

The persisted record is URL + agent + workspace + first-seen. State comes from
`knot-forge` into an in-memory cache with the `diff_stats.rs` claim-refresh
shape: a per-URL TTL, `claim_refresh` handing out a writer, the writer flipping
a dirty flag the repaint poll reads.

- *Why not persist state*: a merged pull request shown as open after a restart is
  worse than a blank, and there is no way to know a cached state is still true.
  The spec says so; this is the mechanism.
- *Why reuse the cache shape rather than a new one*: `.claude/rules/rust-structure.md`
  says to route through `diff_stats.rs` rather than adding a fourth instance of
  this pattern. Whether the existing type generalizes over its value type or a
  sibling is written next to it is an implementation call for the apply phase;
  either way the polling and dirty-flag discipline is the same.
- Refresh runs only while the Pull Requests view is showing. A workspace with
  fifty recorded pull requests should not be polling `gh` fifty times while the
  user is doing something else.
- `WorkspaceWindow` already has a tokio runtime and a repaint poll, and the view
  lives in that window, so fetches go through `spawn_blocking` there. No
  Command-Center-shaped second path is needed.

### The record lives in its own document

A `SavedPullRequest` collection in a sixth settings document rather than a field
on `SavedAgent`.

- *Why*: it grows without bound and is rewritten on a different cadence than the
  agent list. The store's own rule is that a write touches only the document it
  belongs to; putting pull requests on `SavedAgent` would rewrite `agents.json`
  every time a URL scrolls past.
- Cascade deletes (agent removed, workspace removed) are done in
  `knot-agents`'s store alongside the existing removal paths, so there is one
  place that knows an agent is going away.

### The view is a content-area takeover, like the dashboard

A launcher row in the sidebar and a pane that replaces the window's content,
following the Dashboard row's shape (`dashboard` spec) and the markdown/mermaid
takeover pattern in `workspace_window/panel/pane.rs`.

- *Alternative - a badge and popover on each agent card*: rejected. The user
  asked for a list they can see and open from; a per-card popover buries the
  cross-agent view, which is the one that answers "what did this session
  produce".
- *Alternative - a separate window*: rejected. It would need its own registry
  entry, its own bounds and its own refresh loop, for a list scoped to one
  workspace that the workspace window is already showing.

### Opening the URL

`open_in.rs` gains `open_url(&str) -> bool` beside `run_open`, calling
`/usr/bin/open <url>` with the same fail-quietly, macOS-only, `false`-elsewhere
shape. No new dependency.

## Risks / Trade-offs

- **A URL an agent quotes from somewhere else is recorded as one it opened** →
  accepted and specified; the user can remove a row, and removals persist.
- **Scanning every PTY byte is on a hot path** → the scan is a byte-pattern
  search with an early-out on the absence of `/pull/`, run on the same thread
  that already parses the bytes into cells; it is cheap relative to VT parsing.
  Worth a benchmark against a `yes`-style flood before this is considered done.
- **A URL split across a PTY chunk boundary is missed** → the carry buffer
  handles it; a test that feeds a URL one byte at a time is the check.
- **`gh` is slow or hangs** → the runner's timeout, copied from `knot-git`'s
  30s default, bounds it; a timed-out fetch is a failed fetch, which the spec
  already covers without removing the record.
- **`gh pr view --json` field names change between `gh` versions** → the decode
  is tolerant: unknown fields ignored, missing fields leave that part of the
  state absent rather than failing the record. A version of `gh` too old to know
  a field degrades to a partial row.
- **GitHub Enterprise hosts cannot be distinguished from any other host by URL
  shape alone** → the pattern matches `github.com` plus hosts `gh` reports as
  configured; a URL on an unknown host is not recorded. This may miss an
  Enterprise host `gh` does not know about, which is the safe direction.
- **Two windows for one workspace would mean two refresh loops** → the
  `window-and-menu-fixes` change makes that impossible. This change does not
  depend on it landing first, but the two are cheaper in that order.
