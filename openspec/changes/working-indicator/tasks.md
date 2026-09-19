# Tasks

## 1. Shared Indicator Component (gpui-kit layer)

- [ ] 1.1 Create the single `WorkingIndicator` view/component in the gpui-kit
      shared UI layer and verify `cargo +nightly fmt` + `cargo clippy` pass on
      the workspace
- [ ] 1.2 Add the shared status→state mapping (Working / Idle / Awaiting input /
      Error / not-running → indicator state) in one module and verify unit
      tests cover every mapping + the not-running case
- [ ] 1.3 Implement the indicator's animation via gpui-kit's timer/animation
      primitives and verify it animates on Working and Awaiting input, with
      Idle steady and Error distinct (no custom tokio loop)

## 2. Workspace Sidebar Row (agent-list-ui)

- [ ] 2.1 Render the indicator on the agent row beside the existing state dot
      and verify it updates live when `activity-detection`'s status changes
- [ ] 2.2 Add the "not running" visual state and verify a not-running agent
      reads as off (distinct from running-but-idle), with no regression to the
      existing state dot semantics

## 3. Dashboard Agent Card (dashboard)

- [ ] 3.1 Render the same indicator on the dashboard's agent card and verify
      it stays in lockstep with the sidebar row (no drift between surfaces)
- [ ] 3.2 Wire the card's indicator to the shared component and verify the
      per-agent status transitions (Working/Idle/Awaiting input/Error) all
      show correctly on the card

## 4. Verification

- [ ] 4.1 Run `openspec validate` on the change and verify all artifacts pass
      (proposal, specs, design, tasks)
