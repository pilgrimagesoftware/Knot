# Design

## Context

See proposal.md - Why. What the code does now, and the one constraint that
shapes the work:

- `Render for AgentEditor` composes four sections in a fixed order -
  `identity_rows`, `agent_rows`, `registry_rows`, `folder_rows` - as four
  `dialog_section` children of one `v_flex`. Reordering is moving a line.
- `choose_folder` opens the native picker and awaits the result in
  `cx.spawn(async move |_this, cx| ...)`, then writes `editor.folder_path`
  through `cx.update(|app| ...)`. That closure has an app context and **no
  window**.
- `InputState::set_value` requires `&mut Window` at every one of its eleven
  call sites in this workspace. There is no window-free way to put text in a
  field, which is the whole technical content of this change.
- `AgentStore::create` derives a name from the folder with
  `helpers::last_path_component`, which is `pub(super)` inside
  `knot-agents`'s store module and therefore unreachable from `knot`.
- `can_submit` and `validated_fields` already require the folder, and
  `validated_fields` already checks it first. Neither needs to change.

## Goals / Non-Goals

**Goals:**

- The derived name is in the field, editable, before submit.
- One definition of "a folder's name", not two.

**Non-Goals:**

- Reworking how the editor stores or validates its form. The name stays an
  `InputState`; the folder stays a `String`.
- Making the fill configurable, or explaining it with new UI text.

## Decisions

### Thread the window into `choose_folder` and use `spawn_in`

`choose_folder(&mut self, cx)` becomes
`choose_folder(&mut self, window: &mut Window, cx)`, awaits with
`cx.spawn_in(window, async move |this, cx| ...)`, and writes through
`this.update_in(cx, |editor, window, cx| ...)`, which hands back the window
`set_value` needs. This is the pattern gpui itself uses for exactly this
shape - see `hover_card.rs` in `gpui-base`, which spawns with a window and
updates with one.

The call site already has a window it is discarding:
`cx.listener(|editor, _, _, cx| editor.choose_folder(cx))` binds the window
parameter to `_`. Threading it is deleting an underscore.

Alternatives rejected:

- **Set a `pending_name_from_folder` flag and fill on the next render.**
  Render has a window, so this works, but it puts a form mutation on the
  render path and leaves a window of frames where the model and the field
  disagree. The project already treats the render path as somewhere work does
  not belong.
- **Skip the field and default the name in `validated_fields` at submit.**
  Smaller, and wrong: the spec requires the user to see and be able to edit
  the name before submitting. A name that appears only in the created agent is
  a hidden substitution, which is the behaviour the change exists to replace.

### Move `last_path_component` to `knot-core`, and have both callers read it

The editor needs the same rule `AgentStore::create` uses. `knot-core` is a
dependency of both `knot` and `knot-agents`, so the function moves there and
`knot-agents`'s helper becomes a re-export or a direct call.

`Path::file_name` inline in the editor was the alternative. It is three lines,
which is exactly why it is the wrong call: two definitions that agree today
and are not required to keep agreeing, for a rule the user sees as "the agent
is named after its folder". The project's own convention is one roster per
open vocabulary rather than the same list written out twice; this is the same
argument at a smaller scale.

Widening `knot-agents`'s helper to `pub` was also rejected: it publishes a
store-internal module's helper as a crate API to serve a caller that is not
about the store at all.

### Blankness is `trim().is_empty()`, matching validation

The fill triggers on the same test `validated_fields` already uses to reject a
name, so "the dialog would have refused this" and "the dialog fills this in"
can never disagree. A whitespace-only name is blank to both.

The fill is unconditional on mode: it is written as "blank name plus chosen
folder", not "create mode only". Editing an existing agent cannot reach it in
practice, because an agent that exists has a name - and if a user clears the
name and picks a new folder, filling it is the same helpful behaviour rather
than a special case to suppress.

### A folder with no last component leaves the field alone

`Path::file_name` returns `None` for a root or a path ending in `..`. The fill
does nothing then, rather than writing an empty string - the user is left with
the blank field they already had and the normal "enter a name" error, which is
a state the dialog already handles.

## Risks / Trade-offs

- **Threading a window changes a public-ish signature.** `choose_folder` is
  `pub(super)` with one call site, so the compiler finds it. Low cost, named
  here only because it is the one signature change.
- **Moving `last_path_component` touches `knot-agents` for a `knot` feature.**
  It is a pure move with its tests, and the alternative is a duplicate rule.
  Keep the move in its own commit so a bisect can separate it from the
  behaviour change.
- **Reordering sections is invisible to tests.** Nothing asserts section
  order, and a render test that pinned it would be pinning layout rather than
  behaviour. This is the one part of the change that rests on looking at the
  dialog; the spec's scenario says what to look for, and it belongs in the
  manual walk rather than in a test that would break on any re-layout.
- **The fill lands asynchronously.** The picker's result arrives after an
  await, so a user who types a name *while* the picker is open would have it
  preserved (the field is no longer blank by then) - which is the desired
  outcome, but it means the trigger reads the field at completion time, not at
  picker-open time. Read it inside `update_in`, not before the spawn.
