# Tasks

## 1. Pin the defect

- [ ] 1.1 In `crates/knot-agent-launch/src/registration.rs`, add tests
      asserting that the knot instructions order the two duties (current task
      before a teammate's request, the request named as queued work), carve out
      a shared folder as the agent's to work in, and bound replying. Verify
      they fail against the current text - that failure is the defect,
      reproduced.

## 2. Rewrite the instructions

- [ ] 2.1 Replace the `knot_instructions` format string: order the duties,
      swap the project-ownership handoff test for a folder test with the
      shared-folder carve-out, and bound the reply loop. Keep the agent ID, the
      MCP server name and its rationale, all four tool names, the `set-status`
      paragraph and the single line. Fund it by compressing wording elsewhere
      so the render stays under 1,000 characters. Verify the tests from 1.1
      pass.
- [ ] 2.2 Update the doc comment on `knot_instructions` to record why the
      ordering, the folder test and the reply bound are there, in the same
      form as the existing paragraphs - each one names the failure it prevents.

## 3. Repair the tests the rewrite invalidates

- [ ] 3.1 Rewrite `instructions_tell_an_agent_to_hand_work_to_its_knot` and
      `instructions_name_the_mcp_server_the_tools_come_from` to pin the
      behaviour the removed phrases carried rather than the phrases, so the
      MCP-collision rationale and the outward push stay covered. Verify
      `make test` passes.

## 4. Contract

- [ ] 4.1 Add "Knot instructions given to a launched agent" to
      `openspec/specs/agent-launch-command/spec.md` via this change's delta.
      Verify `openspec validate --strict` passes.

## 5. Gate

- [ ] 5.1 Run `make` and confirm the full gate passes.
