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

#### Scenario: Close all closes the panel

- **WHEN** the user activates close-all on a panel with both sections open
- **THEN** both the markdown file and the diagram are closed and no panel is
  shown

#### Scenario: The toolbar is localized

- **WHEN** the panel is drawn
- **THEN** its title and each control's tooltip come from localization keys

### Requirement: The expand control grows the panel to the full content area

The panel SHALL carry an expand control that grows it to occupy the whole
content area, and that returns it to its set width when activated again. The
control SHALL show which of the two states the panel is in.

Expanded state SHALL be tracked per agent. The panel SHALL open expanded when
the artifact was shown with `display-markdown`'s `maximized` argument set, and
SHALL return to the unexpanded state when the panel closes, so a later artifact
shown without `maximized` opens at the set width.

While expanded the panel's drag handle SHALL have nothing to set and SHALL NOT
be shown; the width it had SHALL be kept and restored on returning.

This diverges from the Swift reference, which tracks one expanded flag for the
whole window, so expanding one agent's panel expands the next agent's too.

#### Scenario: Expanding and returning

- **WHEN** the user activates the expand control and then activates it again
- **THEN** the panel fills the content area and then returns to the width it had

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

The two sections and the divider together SHALL fill the panel's height exactly,
at every split: no gap below the lower section and no section clipped by the
panel's edge.

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
