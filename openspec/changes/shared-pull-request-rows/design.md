# Design

## Context

See proposal.md - Why. The store keeps one record per agent per URL and sorts a
workspace's records newest first; the view is the only place that decides how
they are shown, so the fix is confined to it.

## Decisions

### Group by owner set, not into one "shared" bucket

A URL recorded by agents A and B and another recorded by B and C would share a
single "shared" group only under a heading that is wrong for both rows. Keying
the group on the set of owning agents keeps every heading true: each group says
exactly who opened the rows beneath it. In the common case - one set of agents
working one pull request - it is one group either way.

### The grouping is a pure function over records

`pull_request_groups` needs the store lock and the state cache, and neither is
testable without GPUI around it. The decision itself - which URL goes in which
group, in which order - is pulled out into `pull_request_groups::group_records`,
which takes the workspace's records and returns owner sets and URLs. The view
filters out records whose agent has gone before it calls it, and joins names and
states afterwards.

### A shared row's time is its earliest sighting

The row is the pull request, not any one agent's sighting of it, and the pull
request existed from the first time anyone saw it. Taking the latest would let a
long-shared pull request jump to the top each time another agent links it.

### Group order: shared first, then by newest row

Shared groups first, so a pull request several agents worked on - usually the
one that matters - is not hunted for. Within each tier groups keep today's rule:
the group holding the newest row comes first.

### Removal removes every attribution

The row carries its owning agents, and removing it forgets the record for each
of them. The alternative - dropping one attribution so the row falls back into
the remaining agent's group - makes a row the user removed reappear under a
different heading. The issue recommended this and left it to the repo owner;
it is implemented as recommended, and reverting it is local to
`remove_pull_request`.

### Owner order within a heading

Owners are ordered by agent id inside the pure function, so the group key is a
stable value; the view sorts the names alphabetically for the heading, so the
heading does not depend on ids the user never sees.

## Risks / Trade-offs

- **Joined names are not localized** → the heading joins names with ", " inside
  the existing `pull_requests.opened_by` string, as `agent_editor/window.rs`
  already joins a list for display. A localized list join is a separate change.
