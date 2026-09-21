# Design

## Context

Agent activity already has a shared state model with a Working state, while
application quit currently has a direct path. The warning must cover menu,
keyboard, and window-close quit requests without changing agent teardown.

## Goals / Non-Goals

**Goals:**

- Route every application quit request through one guard.
- Derive the warning from the existing agent state store.
- Keep the confirmed quit path separate from the guarded entry point.
- Use localized text and native confirmation-dialog behavior.

**Non-Goals:**

- Automatically stopping or waiting for agents.
- Warning on closing an individual agent or workspace.
- Persisting a user's choice or adding a suppress-warning setting.

## Decisions

- Add the check at the application quit boundary rather than at individual menu items. This covers all quit sources and avoids duplicated state checks.
- Count agents whose current state is Working. Idle, stopped, waiting, and completed states do not trigger the warning because they have no active work to lose.
- Use a one-shot bypass for the confirmation action, then invoke the existing shutdown routine. This avoids recursive prompting while preserving the normal cleanup path.
- Keep the dialog model-owned and update it through the app state so a quit request can be cancelled without tearing down sessions.

## Risks / Trade-offs

- [A quit source bypasses the shared boundary] -> Audit menu, keyboard, window-close, and system termination paths and route them through the same guard.
- [Working state changes while the dialog is open] -> Treat the dialog as a snapshot of the request; the user's explicit choice controls the result.
- [A stale state triggers an unnecessary warning] -> Reuse the central state store and clear Working state through existing activity updates.

## Migration Plan

No data migration is required. Add the guard and localized strings, then verify idle, single-agent, multi-agent, cancel, and confirm paths.
