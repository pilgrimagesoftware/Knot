# Design

## Context

See proposal.md — Why. The mechanics that decide the approach:

- `select_agent` (`workspace_window/agents.rs:55`) takes `&mut self` and nothing
  else. Its four callers are click and menu handlers — the sidebar row, the
  overview card, agent creation, and the New Companion menu item — and the
  window-open path sets `selected_agent` directly rather than calling it, so
  restoring a selection does not go through it at all.
- `prepare_frame` (`workspace_window/render/mod.rs:105`) already does frame-time
  focus work, guarded by `window.focused(cx).is_none()`, and already has both
  `window` and `cx`. It is also where the window's other "what changed since
  last frame" work lives.
- The composer is one `TextareaState` entity per agent, in
  `panel_prompt_inputs`, created on demand by `panel_prompt_input(id, window, cx)`
  and torn down with the session. Focusing it is `state.focus(window, cx)` —
  exactly what `take_queued_prompt_into` already does.
- Whether the composer is on screen is not simply "the agent is Panel mode". The
  content pane (`render/content.rs:80-140`) resolves, in order: a markdown pane,
  then a diagram pane, then the stopped placeholder for a deactivated agent,
  then the panel, then the terminal grid. Only the fourth shows a composer, and
  the window showing its dashboard shows none at all.
- `root_focus` exists so the Agents menu's items sit on the dispatch path macOS
  validates against, and its doc comment records that root focus "leaves them
  there once a pane takes focus, since the root is that pane's ancestor".

## Goals / Non-Goals

**Goals:**

- One place that decides where focus goes, so the four selection paths cannot
  disagree and the window-open path is not a fifth case.
- Take focus on a transition, never hold it, so the rule "focus is not pulled
  back" is structural rather than a series of guards.

**Non-Goals:**

- Focusing the terminal pane. See proposal.md — Non-goals.
- Any general focus-management layer. This is one transition in one window.

## Decisions

### Decide focus at frame preparation, not in `select_agent`

`prepare_frame` computes which agent's composer the frame is about to show — the
same conditions `render/content.rs` uses to choose the panel pane — and compares
it with the one it focused last.

The alternative is to focus inside `select_agent`. That means threading `window`
and `cx` through a method that needs neither, updating four call sites, and
still missing the window-open path, which does not call it. It also focuses
before the frame that shows the pane, so the conditions it would have to check
are conditions about a layout that has not been computed yet.

Doing it at frame preparation also makes the pane-precedence rules fall out
rather than being restated: the frame already knows a markdown pane took the
content area, because that is the decision it is about to make.

### One field: the agent whose composer was last focused

`WorkspaceWindow` gains `focused_composer: Option<Uuid>`. Each frame:

- Compute `showing: Option<Uuid>` — the selected agent when the frame will draw
  its panel pane with a composer, `None` otherwise.
- If `showing != focused_composer`, and `showing` is `Some(id)`, focus that
  agent's composer.
- Store `showing` either way.

Storing `None` when no composer is showing is deliberate: a markdown pane
opening and then closing over the same agent returns to the conversation, and
the composer should be focused again when it does. That is the same transition
as selecting the agent, and treating it differently would need a reason.

Because the comparison is against the *showing* id and not against where focus
actually is, the user moving focus elsewhere does not change anything the next
frame compares, so focus is not taken back. That is the whole mechanism for the
"not pulled back" requirement — there is no second guard to keep in step.

This is the same shape as `last_spinner_frame` in the same struct: a per-frame
comparison that says whether a transition happened. Add it to `teardown_session`
alongside the other per-agent state, per the parallel-maps rule — a stale id
here is harmless but the field is per-agent and the teardown is one function by
design.

### Guard the dialog case by asking whether a dialog is open

Before focusing, require that no dialog is open: `!window.has_active_dialog(cx)`.

**This replaces the containment guard this section originally specified**, which
does not work. The original read was that `app_support::root_overlays` adds the
dialog layer as a sibling of the view's own tree, so a trapped dialog focus
would fail `self.root_focus.contains_focused(window, cx)`. It is not a sibling:
`root_overlays` is passed to `.children(..)` on the very element that carries
`.track_focus(&self.root_focus)` (`render/mod.rs`), and `anchored()` positions
the dialog without moving it in the dispatch tree. The dialog's focus handle is
therefore a descendant of `root_focus`, and containment answers `true` with a
dialog open — it cannot tell a dialog apart from the window's own panes.

`tests/composer_focus.rs::an_open_dialog_sits_inside_the_root_focus_subtree`
is what establishes this. It asserts both halves, because only the pair is
evidence: that opening a dialog moves focus off the root at all, and that focus
is still contained in the root's subtree afterwards. It stays in the suite as a
guard on the dependency — if a gpui-component upgrade ever moves the layer out,
that test fails and this decision can be revisited.

The rejected alternative was to ask `Root` directly, on the grounds that the
only entry point is `render_dialog_layer`, which builds the layer as a side
effect of answering. That objection does not apply to the API actually used
here: `WindowExt::has_active_dialog` is a plain read —
`Root::read(self, cx).active_dialogs.len() > 0` — and renders nothing. (The
field itself is `pub(crate)` in gpui-component, so this method is the only way
to reach it from Knot, and it is the right one.)

### Ordering against the `root_focus` guard

The composer focus runs before the existing `window.focused(cx).is_none()` root
guard, and the guard then sees focus as taken and does nothing — which is
correct: the menu handlers are declared on the root element and the composer is
its descendant, the same relationship the terminal pane already has. The
existing comment says so; this change relies on it, so the Agents menu with a
focused composer is worth checking rather than assuming.

## Risks / Trade-offs

- **The Edit menu becomes enabled far more often.** `app-menu` requires its five
  items enabled exactly when a text field has focus, and a focused composer is
  one. This is correct by that requirement and is a visible behavior change:
  ⌘C in a panel now copies from the composer where previously nothing claimed
  it. No spec changes; check it does what the requirement says.
- **Focus moves under a user who was mid-keystroke.** Selection is a deliberate
  act, and the window's own dashboard and panes are the only things focus could
  be taken from — never another window, since this sets focus inside a window
  and never raises one. The containment guard is what keeps a dialog out of it.
- **Composers are created per agent, and this creates one on selection.**
  `panel_prompt_input` already gets-or-creates on the render path for the same
  agent a frame later, so this moves the allocation earlier by one frame for an
  agent the user just selected. It does not create composers for agents the user
  has not opened.
- **A frame-time comparison means focus lands one frame after the click.** Not
  perceptible, and it is the same frame the pane itself appears in — focusing
  earlier would focus something not yet on screen.
