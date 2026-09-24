# Spec Delta

## Purpose

Holds the artifacts an agent puts in front of the user — a markdown file shown
with `display-markdown`, a diagram shown with `view-mermaid` — in one side panel
beside the agent's conversation or terminal, so both can be open at once and
neither hides the session that produced them.

## ADDED Requirements

### Requirement: The artifact panel holds both of an agent's artifacts

The system SHALL show an artifact panel for the selected agent whenever that
agent has a markdown file open, a diagram open, or both. The panel SHALL hold a
markdown section for the file and a mermaid section for the diagram, and SHALL
show a section only while its artifact is set.

The panel SHALL be gone when the agent has neither, and SHALL appear without any
further action when either is set — an agent calling `display-markdown` or
`view-mermaid` on the selected agent SHALL see its artifact on screen.

Each agent SHALL have its own panel: selecting a different agent SHALL show that
agent's artifacts, or no panel if it has none. The first agent's artifacts SHALL
still be there on returning to it.

This diverges from the Rust port before this change, which drew the markdown
pane and the diagram pane as alternatives and tried the markdown pane first, so
a diagram shown while a markdown file was open was stored and never drawn.

#### Scenario: Both artifacts are open

- **WHEN** the selected agent has a markdown file open and is then shown a
  diagram
- **THEN** the panel shows a markdown section and a mermaid section together

#### Scenario: A diagram arrives for an agent with a file open

- **WHEN** an agent with a markdown file open calls `view-mermaid`
- **THEN** the mermaid section appears in the panel alongside the markdown
  section, and the markdown section still shows its file

#### Scenario: An agent with no artifacts has no panel

- **WHEN** the user selects an agent with neither a markdown file nor a diagram
- **THEN** no artifact panel is shown

#### Scenario: Artifacts follow the agent

- **WHEN** the user selects a second agent with no artifacts, and then the first
  agent again
- **THEN** no panel is shown for the second, and the first agent's panel returns
  with the same sections open

### Requirement: The panel sits beside the content, not over it

The artifact panel SHALL be a sibling of the content pane, laid out at the
trailing edge of the content area. Opening it SHALL narrow the conversation or
terminal rather than replace, cover or occlude it. The user SHALL be able to
read a shown artifact and prompt the agent about it without closing either.

When the git panel is also open, both SHALL be shown, with the artifact panel
outermost — the content pane, then the git panel, then the artifact panel —
matching the Swift reference's order.

When the window shows its dashboard or its pull requests instead of an agent,
the panel SHALL stay shown beside them. The dashboard covers the content pane,
not the whole window, and the selected agent's artifacts are still that agent's;
closing the panel because the user glanced at the overview would throw away what
an agent put in front of them. This matches the Swift reference, which keeps the
artifact panel over a visible dashboard and force-closes only the git panel, and
only when the window's set of active agents changes.

This diverges from the Rust port before this change, where either pane took the
whole content area, hiding the conversation and its composer.

#### Scenario: The conversation stays readable

- **WHEN** an agent shows a markdown file while its conversation is on screen
- **THEN** the conversation is still shown, narrowed, with the panel beside it

#### Scenario: The composer stays usable

- **WHEN** a Panel-mode agent has a markdown file open
- **THEN** the user can type and send a prompt without closing the panel

#### Scenario: Both panels open

- **WHEN** an agent has the git panel open and then shows a diagram
- **THEN** the content pane, the git panel and the artifact panel are all shown,
  in that order from the leading edge

#### Scenario: The dashboard does not close the panel

- **WHEN** the selected agent has a markdown file open and the user switches the
  window to its dashboard
- **THEN** the artifact panel is still shown beside the dashboard

### Requirement: The panel's width is set by dragging its edge

The panel SHALL carry a drag handle on its leading edge that sets its width.
Dragging SHALL widen or narrow the panel and narrow or widen the content pane by
the same amount, and SHALL clamp the width to no less than 350 and no more than
800 points. A panel the user has not resized SHALL be 500 points wide.

The width SHALL be tracked per agent, so resizing one agent's panel SHALL NOT
resize another's.

The width SHALL NOT be persisted. Closing the window SHALL discard it, and the
panel SHALL open at the default width in a new window. This matches both the
Swift reference and the git panel, whose width is tracked the same way.

#### Scenario: Dragging the handle resizes the panel

- **WHEN** the user drags the panel's leading edge toward the leading edge of
  the window
- **THEN** the panel gets wider and the content pane gets narrower

#### Scenario: The width is clamped

- **WHEN** the user drags the handle past either limit
- **THEN** the panel stops at 350 points at the narrow end and 800 at the wide
  end

#### Scenario: Width is per agent

- **WHEN** the user widens one agent's panel and then selects another agent that
  has an artifact open
- **THEN** the second agent's panel is at its own width, not the first's

#### Scenario: Width does not outlive the window

- **WHEN** the user resizes the panel, closes the workspace window, and opens it
  again
- **THEN** the panel is at the default width

### Requirement: The panel toolbar names the panel and closes it

The panel SHALL carry a toolbar above both sections holding a localized title
for the panel, an expand control, and a close-all control. Activating close-all
SHALL close both sections at once, which closes the panel.

Every control in the toolbar SHALL carry a localized tooltip naming what it
does, and the title and tooltips SHALL be resolved through localization rather
than carried as literal English.

Each section SHALL carry its own close control in its header, closing that
section's artifact and leaving the other's alone. This is what close-all does to
both at once. Every section control SHALL be localized on the same terms as the
toolbar's: the diagram section's close control is literal English today and SHALL
NOT stay so.

#### Scenario: Close all closes the panel

- **WHEN** the user activates close-all on a panel with both sections open
- **THEN** both the markdown file and the diagram are closed and no panel is
  shown

#### Scenario: Closing one section leaves the other

- **WHEN** the user activates the markdown section's own close control on a
  panel showing both
- **THEN** the markdown file is closed and the diagram is still shown

#### Scenario: The panel is localized

- **WHEN** the panel is drawn with both sections open
- **THEN** its title and every control's tooltip, in the toolbar and in both
  section headers, come from localization keys

### Requirement: The expand control gives the panel the whole content area

The panel SHALL carry an expand control that grows it to take the whole content
area, and that returns it to its set width when activated again. The control
SHALL show which of the two states the panel is in.

While expanded, the content pane SHALL be collapsed to no width, hidden, and
SHALL NOT accept clicks or keystrokes; the conversation or terminal surface it
holds is not on screen. The sidebar and an open git panel SHALL be unaffected —
the panel takes the content pane's space and nothing else.

Because the content pane is not on screen while expanded, `acp-panel-ui` and
`terminal-input` SHALL withhold focus for exactly that case, and for no other
state of this panel. An unexpanded panel leaves the composer or terminal surface
on screen beside it and SHALL NOT withhold focus.

Expanded state SHALL be tracked per agent, and SHALL be taken from
`display-markdown`'s `maximized` argument whenever that agent's markdown file or
that argument differs from the pair the panel last took — not once when the
panel first opens.

That trigger is what makes the argument mean something on every call that says
something new. An agent that shows one file without `maximized` and then another
with it SHALL end up expanded; so SHALL an agent that re-shows the file already
open, this time asking for it maximized. An agent that re-shows the same file
with the same argument SHALL change nothing, so a panel the user expanded by
hand SHALL survive an agent repeating itself — re-showing a file it has just
edited is ordinary behaviour and is not an instruction about the panel's size.

Expanded state SHALL return to unexpanded when the panel closes — when the agent
has neither artifact left — and SHALL NOT be changed by closing one section
while the other is still open.

The `maximized` argument SHALL be the only writer of that state other than the
control itself. Activating the control SHALL change what is on screen and SHALL
NOT write back into the agent's stored panel state, so a control the user
toggled cannot be mistaken for an instruction the agent gave.

While expanded the panel's drag handle SHALL have nothing to set and SHALL NOT
be shown; the width it had SHALL be kept and restored on returning.

This diverges from the Swift reference only at the edges. Swift holds one
expanded flag for the whole window, but re-seeds it as the selected agent's
markdown file changes, so per-agent behaviour emerges for agents that have a
markdown file. The flag carries over from the previously selected agent in two
cases it does not cover: when the newly selected agent has no markdown file but
does have a diagram, and when both agents have the same file open. Tracking the
state per agent removes both.

#### Scenario: Expanding and returning

- **WHEN** the user activates the expand control and then activates it again
- **THEN** the panel takes the whole content area, and then returns to the width
  it had

#### Scenario: An expanded panel hides the content pane

- **WHEN** an agent's panel is expanded, by the control or by `maximized`
- **THEN** the content pane is collapsed to no width and takes no clicks, and
  the sidebar and any open git panel are still shown

#### Scenario: A later file asks to be maximized

- **WHEN** an agent shows one markdown file without `maximized` and then a
  second file with it
- **THEN** the panel is expanded, rather than staying as the first file left it

#### Scenario: Closing one section does not collapse the panel

- **WHEN** an agent's panel is expanded with both sections open and the user
  closes the markdown section
- **THEN** the panel is still expanded, showing the diagram

#### Scenario: An agent asks for a maximized file

- **WHEN** an agent calls `display-markdown` with `maximized` set
- **THEN** that agent's panel opens expanded

#### Scenario: Expanded state is per agent

- **WHEN** the user expands one agent's panel and selects another agent with an
  artifact open
- **THEN** the second agent's panel is not expanded

#### Scenario: Expanded state does not carry to the next artifact

- **WHEN** the user expands an agent's panel, closes all its sections, and the
  agent shows another file without `maximized`
- **THEN** the panel opens unexpanded

### Requirement: A divider splits the two sections and can be dragged

When both sections are open and neither is collapsed, a draggable divider SHALL
sit between them and set how the panel's height is divided. Dragging it SHALL
grow one section and shrink the other by the same amount, and SHALL clamp the
split so that neither section takes less than 15% or more than 85% of the
available height. A split the user has not dragged SHALL divide the height
evenly.

The two sections and the divider together SHALL fill the panel's section area
exactly, at every split: no gap below the lower section and no section clipped
by the panel's edge. The section area is what the panel has left below its
toolbar — the toolbar sits above both sections and is not divided by the split,
so it SHALL NOT be counted in the height the two sections share.

The split SHALL be tracked per agent and SHALL NOT be persisted, on the same
terms as the panel's width.

#### Scenario: Dragging the divider changes the split

- **WHEN** the user drags the divider downward
- **THEN** the markdown section is taller and the mermaid section shorter by the
  same amount

#### Scenario: The split is clamped

- **WHEN** the user drags the divider to either end of the panel
- **THEN** the smaller section stops at 15% of the available height

#### Scenario: The sections fill the panel

- **WHEN** the divider is at any position within its range
- **THEN** the two section heights plus the divider's height equal the panel's
  height

### Requirement: Either section can be collapsed to its header

When both sections are open, each section's header SHALL carry a collapse
control that shows whether the section is collapsed, and that toggles it.

A collapsed section SHALL show its header alone and SHALL give the rest of its
height to the other section, which SHALL take everything the panel has but the
collapsed header. With both sections collapsed the panel SHALL show two headers
and nothing else.

No divider SHALL be drawn while either section is collapsed: there is no split
left to set.

Collapsing SHALL NOT close a section or discard what it holds. Expanding it
again SHALL show the same file or diagram, and closing the section SHALL remain
a separate action with its own control.

Collapse state SHALL be tracked per agent and SHALL NOT be persisted, on the
same terms as the panel's width and split. This matches the Swift reference,
which does not persist it either, and the panel's existing refusal to persist
tool-call collapse state.

#### Scenario: Collapsing one section

- **WHEN** the user collapses the markdown section
- **THEN** the markdown section shows its header only and the mermaid section
  takes the rest of the panel's height

#### Scenario: No divider while collapsed

- **WHEN** either section is collapsed
- **THEN** no divider is drawn between them

#### Scenario: Both collapsed

- **WHEN** the user collapses both sections
- **THEN** the panel shows the two headers and no content

#### Scenario: Collapsing keeps the content

- **WHEN** the user collapses a section and then expands it again
- **THEN** the same artifact is shown, unchanged

#### Scenario: Collapse does not survive the window

- **WHEN** the user collapses a section, closes the workspace window and opens
  it again
- **THEN** both sections are expanded

### Requirement: A single open section fills the panel

When only one section is open, it SHALL fill the panel's height. No divider
SHALL be drawn and that section SHALL NOT carry a collapse control: there is
nothing to split and no other section to give height to.

Closing one of two open sections SHALL leave the other filling the panel, and
SHALL discard the collapsed state of the section that closed, so reopening it
shows it expanded.

#### Scenario: Only a diagram is open

- **WHEN** an agent has a diagram open and no markdown file
- **THEN** the mermaid section fills the panel, with no divider and no collapse
  control

#### Scenario: Closing one of two sections

- **WHEN** the user closes the markdown section of a panel showing both
- **THEN** the mermaid section fills the panel, and its collapse control is gone

#### Scenario: A reopened section is not collapsed

- **WHEN** the user collapses the markdown section, closes it, and the agent
  shows another markdown file
- **THEN** the markdown section is shown expanded

### Requirement: Panel arrangement is discarded with the window

None of the panel's arrangement — its width, its split, which sections are
collapsed, whether it is expanded — SHALL be written to the settings store. It
SHALL live only for as long as the workspace window that shows it, and SHALL be
discarded when an agent is closed, as the git panel's width already is.

A new workspace window SHALL open every agent's panel at the default width, the
even split, both sections expanded and the panel unexpanded.

#### Scenario: Arrangement is not persisted

- **WHEN** the user resizes the panel, drags its divider and collapses a section
- **THEN** no settings document is written on account of any of it

#### Scenario: Closing an agent discards its arrangement

- **WHEN** an agent whose panel was resized is closed and another agent is
  created
- **THEN** the new agent's panel opens at the default width
