# Design

## Context

See `proposal.md` - Why. The constraints that shape the approach, all of them
existing:

- The persisted record (`SavedPullRequest`, `crates/knot-core/src/settings/records.rs`)
  deliberately carries no fetched state: URL, agent id, workspace id and
  `first_seen` (i64 epoch seconds) only. Its doc comment states the record is
  evidence of what was seen, not a copy of the pull request.
- `first_seen` is when an agent's output carried the URL. It is unrelated to
  when the pull request merged.
- Fetched state (`PullRequestState`, `crates/knot-forge/src/pull_request.rs`)
  is never persisted. `crates/knot/src/pull_request_state.rs` holds it in a
  `RefreshCache` keyed by URL that starts empty every launch.
- `gh pr view` is asked for exactly the fields a row renders
  (`PULL_REQUEST_FIELDS`, `crates/knot-forge/src/consts.rs`). `mergedAt` is not
  among them.
- Refreshing is gated on the Pull Requests view being open
  (`refresh_pull_request_states`, `crates/knot/src/workspace_window/pull_requests_view.rs`),
  so a workspace with fifty records is not polling `gh` while the user works
  elsewhere.
- Removing a record already has a settled sequence: mutate `AgentStore` ->
  `persist_pull_requests` -> `prune_pull_request_states`.

## Goals / Non-Goals

**Goals:**

- Decide expiry from the forge's merge time, so "a day after merge" means what
  it says.
- Add nothing to the persisted document, so there is no migration and no
  forward-compatibility question.
- Reuse the existing removal sequence rather than opening a second path that
  mutates records.
- Degrade to "nothing expires" whenever the evidence is missing, never to
  "something was dropped that should not have been".

**Non-Goals:**

- Persisting fetched state. The cache stays in memory.
- A background sweep independent of the view. Expiry needs fetched state, and
  fetching is view-gated by design.
- A confirmation prompt. Expiry is not a user action, and a prompt for
  something the user did not do is noise.

## Decisions

### Measure from the forge's merge time, not from a locally observed one

`PULL_REQUEST_FIELDS` gains `mergedAt`; `RawPullRequest` and
`PullRequestState` gain `merged_at: Option<OffsetDateTime>`, parsed with
`time`'s `Rfc3339` - already a workspace dependency, and the idiom
`crates/knot-history/src/providers/gemini.rs` uses. `knot-forge` gains
`time.workspace = true`; no new third-party dependency enters the workspace.

Alternative considered: stamp a `merged_seen` on `SavedPullRequest` the first
time a refresh reports the status as merged, and measure from that. Rejected on
three counts. It is wrong after a restart - the cache starts empty, so a pull
request merged a week ago would be "first observed merged" today and sit for a
further day. It drifts from the real merge time by up to the refresh interval
even in the good case. And it changes the persisted document's shape, which
costs a field on a struct whose whole point is that it holds no fetched state,
plus the equality-test churn in
`crates/knot-core/src/settings/store/tests/pull_requests.rs`.

The cost of the chosen option is one extra JSON field per `gh pr view`, against
a constant whose comment says it asks for nothing a row renders. The comment's
principle is "do not pay for data nothing uses"; this field is used, by the
expiry rule rather than by a row, so the principle holds and the comment needs
a sentence saying so.

### Expire from the refresh path, gated on the view like the fetch it depends on

The prune runs where the fetched state lands, in the Pull Requests view's
refresh cycle, and therefore only while that view is open.

Alternative considered: a timer that prunes regardless of which view is
showing. Rejected because expiry is decided from fetched state, and fetching
off-view is precisely what the current design refuses. A timer would either
prune from stale cache entries or drag the `gh` polling back out of the gate.

The consequence is that a record which passes the window while the view is
closed is dropped when the user next opens the view, and the launcher row's
merged count can lag until then. This is consistent with what the row already
does: after a restart it shows a total rather than a breakdown, because no
state has been fetched. The lag resolves at the moment the user looks at the
list, which is the only moment a stale row costs them anything.

### Prune in the store, next to the other record removals

`AgentStore` (`crates/knot-agents/src/store/pull_requests.rs`) gains a method
that drops records by predicate, sitting with `remove_pull_request` and the
`forget_*` cascades. The view passes it the URLs it has positively judged
expired; the store does not know about merge times, forges or clocks, and stays
testable without any of them.

The caller then runs the existing sequence: `persist_pull_requests` if anything
was removed, then `prune_pull_request_states` to drop the now-orphaned cache
entries. Both already exist and are already correct for this case.

### The window is a named constant, not a setting

`PULL_REQUEST_MERGED_RETENTION: Duration = Duration::from_secs(24 * 60 * 60)`
in `crates/knot/src/consts.rs`, beside the two existing pull request intervals.
Making it a setting means a settings-UI row, a persisted field and a migration
for a number nobody has yet asked to change. The constant is the smaller change
and does not foreclose the setting.

### Absent evidence keeps the record

Four cases, one rule - keep it:

- state not yet fetched, or fetch failed: nothing to judge;
- status merged but `mergedAt` absent (an older `gh`, or a forge that omits
  it): `Option::None`, no comparison, no prune;
- status open, draft or closed: out of scope by the spec;
- a merge time in the future (clock skew between the machine and the forge):
  the elapsed span is negative, so it is not past the window and the record
  stays. This falls out of the comparison rather than needing a guard, but it
  wants a test so it stays that way.

## Risks / Trade-offs

- **A long-merged pull request appears and then vanishes on its first fetch.**
  An agent linking a pull request that merged a month ago produces a row that
  disappears a moment later. → Accepted: it is the correct reading of the rule,
  and the alternative - a grace period from `first_seen` - turns one rule into
  two and reintroduces the sighting time the first decision just removed.

- **Re-recording churn.** An expired pull request whose URL appears in output
  again is recorded again, then expires again on the next fetch. → This is
  exactly the behavior the spec already mandates for a user-removed record, and
  output is read as it is processed rather than re-read, so it takes genuinely
  new output to trigger. No new mechanism; no tombstone list to keep.

- **The prune runs on the frame.** It must not be a fetch or a file read on the
  render path. → It is a comparison over cached state plus, only when something
  actually expired, the same `persist_pull_requests` write the existing manual
  removal already performs from a callback. The early-out when nothing has
  expired is what keeps the common frame free, and is worth a test.

- **A `gh` that does not report `mergedAt` silently disables the feature.**
  → Deliberate: the spec makes it a requirement that this costs the user
  nothing beyond records that do not expire. Nothing is dropped on a guess.

## Migration Plan

None required. `pull-requests.json` keeps its current shape, `SavedPullRequest`
is unchanged, and fetched state was never persisted. An existing install picks
up the behavior on first launch: the first time the Pull Requests view is
opened, records whose pull requests merged more than a day ago are dropped.

Rollback is removing the prune call; records that were already dropped are not
restored, and are re-recorded if an agent prints their URLs again - the same
position a user who removed rows by hand would be in.

## Open Questions

- Whether closed (unmerged) pull requests should eventually expire too, and on
  what window. Deferrable: it changes no requirement, decision or task here,
  and the prune predicate is the natural place to extend if the answer is yes.
