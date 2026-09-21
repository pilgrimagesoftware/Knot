# Tasks

## 1. Model auxiliary inbox nudges

- [ ] 1.1 Locate the idle delivery and session prompt queue paths, then add an explicit origin for automatic inbox nudges; verify normal user messages still use the existing origin.
- [ ] 1.2 Queue the automatic inbox nudge behind an active turn instead of interrupting it; verify the active state remains unchanged while the nudge waits.
- [ ] 1.3 Update the nudge text to tell the agent to continue its previous work if there is nothing to do; verify the exact fallback instruction is delivered.
- [ ] 1.4 Resume the preserved task after the nudge is acknowledged; verify the task continues with its original turn identity and does not duplicate output.

## 2. Tests and verification

- [ ] 2.1 Add focused tests for a nudge during paused work, a nudge during an active update, and a nudge with no active task; verify each expected queue transition.
- [ ] 2.2 Run the affected MCP/session tests and verify all pass.
- [ ] 2.3 Run repository formatting and broader test checks required by the touched crate and verify no regressions.
