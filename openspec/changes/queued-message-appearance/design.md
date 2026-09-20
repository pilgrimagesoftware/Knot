# Design

## Context

The ACP panel already has theme font and color lookups and renders message
rows with status metadata. The change should use those existing values at the
row boundary so queued content and status remain visually separate.

## Goals / Non-Goals

**Goals:**

- Keep queued rows to one visual line.
- Apply the existing monospace and proportional font families to the correct
  portions of the row.
- Use the theme failure color for `failed` status.

**Non-Goals:**

- No changes to message state, queue delivery, or protocol payloads.
- No new theme tokens or dependencies.
- No typography changes to ordinary messages or expanded content.

## Decisions

- Apply single-line truncation to the queued content text element, not the
  containing card, so the status label remains visible. This preserves the
  status while allowing long content to shrink.
- Resolve the monospace font for queued content and the proportional font for
  status from the existing theme. Hard-coded fonts would bypass user theme
  settings.
- Resolve `failed` color through the existing danger/failure theme token and
  leave other statuses on their current color path. This keeps failure salient
  without adding success coloring.
- Keep the row's status as a separate element from content. Combining the
  strings would make independent truncation and font selection unreliable.

## Risks / Trade-offs

- [Very long unbroken content can still be difficult to scan] -> Ellipsis
  preserves row height and the full content remains available through the
  existing message details/expansion behavior.
- [A theme may not define a distinct failure token] -> Reuse the panel's
  existing danger color fallback rather than introducing a new token.
