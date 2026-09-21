# Design

## Context

See proposal.md - Why. The ACP panel already owns queued prompts and response actions; this change adds one queue mutation and aligns retry with the existing icon-button convention.

## Goals / Non-Goals

**Goals:**

- Remove one pending queue entry without touching active work or neighboring entries.
- Keep queue updates atomic from the panel's point of view.
- Reuse the current icon, tooltip, localization, and action-button patterns.
- Preserve the retry callback and only change its presentation.

**Non-Goals:**

- Queue persistence across restarts.
- Bulk deletion, queue reordering, or message editing.
- New retry semantics.

## Decisions

### Delete by stable queued-message identity

Each queued message action targets its stable queue identifier, and the queue removes the matching entry by identity. This avoids deleting the wrong row when identical text appears more than once.

Alternative: remove by array index. Rejected because streaming and concurrent queue updates can shift indices.

### Keep deletion local to pending state

The delete action mutates only the pending-message collection. The active turn and ACP request remain untouched, and the row disappears after the state update.

Alternative: send a cancellation request to the agent. Rejected because queued messages have not been submitted and cancellation would broaden the behavior.

### Use the standard icon button for retry

Replace the retry text label with the same icon-button primitive used by neighboring panel actions. Supply a localized tooltip and accessibility label, and retain the existing retry handler.

Alternative: add a custom retry control. Rejected because it would create a second button convention.

## Risks / Trade-offs

- [Queue updates race with deletion] -> Apply removal against the latest queue state by stable id and ignore an already-absent id.
- [Icon meaning is unclear] -> Use a familiar retry glyph plus localized tooltip and accessibility label.
- [Localization key is missing] -> Add retry and delete labels to the existing localization table and test fallback behavior.

## Migration Plan

No data migration is required. Ship the delete action and retry presentation together. Rolling back removes the delete affordance and restores the text retry label without changing queued messages or retry behavior.
