# Proposal

## Why

A workspace of seven agents - same project, same folder, near-identical
configuration - drifted into bike-shedding and had to be recovered by
terminating sessions (#447).

The launch prompt is the cause. `knot_instructions` in
`crates/knot-agent-launch/src/registration.rs` tells every agent to "Reach for
your knot first and your own effort second" and to "take on what a teammate
asks of you", and says nothing about what becomes of the work the agent was
already doing. Every agent reads the same instruction, so a request from any
one of them preempts the recipient, whose own task is dropped and never
resumed. With seven agents the preemption is mutual and the knot converges on
discussing work instead of doing it.

The handoff test compounds it. "If the work belongs to a teammate's project"
cannot discriminate when one project is all there is: every agent is in that
project, so either reading - all of it is mine, none of it is - is available,
and the prompt offers nothing to choose between them.

Nothing in the prompt bounds replying, either. An agent that has answered a
teammate has no instruction telling it that the exchange is over, so a
question begets a question and the knot pays for a conversation it never
decided to have.

## What Changes

- The two duties are ordered: the agent's current task outranks an incoming
  request, which is queued work rather than an interrupt.
- The project-ownership handoff test becomes a folder test, with an explicit
  carve-out - a folder you share with a teammate is yours to work in - so the
  single-project workspace has a decidable answer.
- The reply loop is bounded: two conditions for replying are named, and
  opening a design debate, seeking consensus, and waiting for approval before
  acting are forbidden.
- The agent ID, the MCP server name and the rationale for naming it, the four
  tool names, the mandatory `set-status` paragraph and the single-line
  constraint all survive.
- The new instructions are funded by compressing wording elsewhere rather than
  by raising the ceiling: the prompt renders at 999 characters against 948
  today, under the existing 1,000-character budget.
- The existing tests that pinned the removed wording (`Reach for your knot
  first`, `hand it to them`, the verbatim MCP-collision sentence) are rewritten
  to pin the behaviour those phrases carried, and new tests pin the ordering,
  the folder carve-out, and the reply bound.
- A requirement covering the prompt's content is added to
  `agent-launch-command`, which had none: the crate's contract described how
  the prompt is delivered and never what it has to say, which is why an
  instruction that deadlocked a knot passed the gate.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-launch-command`: gains "Knot instructions given to a launched agent",
  requiring the duty ordering, the folder-based handoff test and its
  shared-folder carve-out, the bounded reply, the preserved elements, and the
  character budget.

## Impact

- `crates/knot-agent-launch/src/registration.rs`: the `knot_instructions`
  format string and its doc comment; the tests in the same file.
- No runtime, protocol, or persisted-data change. Running agents keep the
  prompt they were launched with until they are relaunched.

## Non-Goals

- Enforcing any of this at runtime. Knot cannot make an agent finish a task
  before reading a message; the prompt is the only lever, and this change is
  about pulling it correctly.
- Changing `REGISTRATION_USER_PROMPT` or the persona text. Neither took part
  in the deadlock.
- Raising the 1,000-character budget. Every launched agent pays for this text
  in its context window; a fix that costs more context than it saves is not a
  fix.
