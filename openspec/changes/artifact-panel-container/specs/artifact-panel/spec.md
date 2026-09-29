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

This SHALL hold whatever the selected agent is doing. An artifact set for an
agent that is not streaming a turn — a Terminal-mode agent, an idle one, or one
a different agent named — SHALL appear on the next frame rather than waiting for
some unrelated event to redraw the window. These tools are called from the MCP
server, off the thread that draws, so arriving is not the same as being drawn.

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

#### Scenario: An artifact for an idle agent still appears

- **WHEN** one agent calls `display-markdown` naming a second, selected agent
  that is idle and streaming nothing
- **THEN** that agent's panel appears without waiting for anything else to
  happen in the window

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
whole content area whenever it was open, hiding the conversation and its
composer for as long as the artifact was shown. Here that happens only while the
panel is expanded, which is a state the user or an agent asks for.

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

Returning the panel to its set width SHALL NOT take focus. Expanding and
collapsing change what is on screen; they are not a change of selection, and
focus SHALL stay wherever the user last put it. A user who collapses the panel
while typing somewhere else in the window SHALL keep typing there.

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

#### Scenario: Collapsing does not steal focus

- **WHEN** the user selects an agent whose panel is expanded, moves focus
  elsewhere in the window, types, and then returns the panel to its set width
- **THEN** focus stays where the user put it and the keystrokes continue to go
  there, with the composer shown but not focused

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

### Requirement: A closed panel can be reopened

Closing a section - by its own close control or by close-all - SHALL remember
what it held: the markdown file's path, and the diagram's source and title. The
window SHALL keep the last closed markdown file and the last closed diagram for
each agent.

While the selected agent has a closed artifact whose section is empty again, the
agent's header SHALL show a reopen control beside the agent's status, with a
localized tooltip. Activating it SHALL put back every such artifact through the
store, as an agent's own `display-markdown` or `view-mermaid` call would, so the
panel reappears by the same path. The control SHALL NOT be shown when there is
nothing to reopen.

Reopening SHALL NOT replace an artifact the agent has shown since the user
closed its predecessor: a section the agent has refilled is newer than what the
user closed and is left as it is. A reopened markdown file SHALL open
unexpanded. `maximized` is what the agent asked for, and a user reopening a
panel asks for nothing about its size.

What was closed SHALL be window state on the same terms as the panel's
arrangement: not persisted, and discarded when the agent is closed. The Markdown
Files menu, which reads the agent's persisted markdown history, stays the way
back to older files.

This diverges from the Swift reference, which has no way to reopen a closed
panel. There, as in the Rust port before this, the only way back was for the
agent to show the artifact again, and a closed diagram was lost outright.

#### Scenario: Reopening after close-all

- **WHEN** the user activates close-all on a panel showing a markdown file and a
  diagram, and then activates the reopen control
- **THEN** both sections are shown again, with the same file and the same
  diagram and title

#### Scenario: Nothing to reopen

- **WHEN** the selected agent's panel is open with every section it has had, or
  the agent has never had an artifact
- **THEN** no reopen control is shown

#### Scenario: A newer artifact is not replaced

- **WHEN** the user closes a markdown file, the agent then shows a different
  file, and the user activates the reopen control
- **THEN** the newer file is still shown

### Requirement: The markdown section follows its file

The markdown section SHALL NOT read its file on the render path. The file SHALL
be read off the main thread when the section opens, and read again whenever it
changes on disk, and each read that lands SHALL reach a frame through
`repaint_poll_tick` without waiting for an unrelated repaint. Until the first
read lands, the section SHALL show a localized loading state. A file that cannot
be read SHALL show a localized note naming the file and the error in its place.

This matches the Swift reference's `FileWatcher`, and diverges from the Rust
port before this change, which re-read the file on every frame - once per
keystroke in the composer beside it.

#### Scenario: An agent edits the file it showed

- **WHEN** the selected agent's markdown file is open and the agent rewrites it
- **THEN** the section shows the new contents without the agent calling
  `display-markdown` again

#### Scenario: Typing does not read the file

- **WHEN** the user types in the composer beside an open markdown section
- **THEN** the file is not read on account of the keystrokes

### Requirement: The markdown section can be approved or reviewed

While a markdown section is expanded, its header SHALL offer the review
verdicts of the Swift reference's `MarkdownPanelView`, each with a localized
label and tooltip:

- While the user is reading the file, **Approve** and **Review**.
- **Approve** SHALL send the agent the message `approved let's do it` as one
  submitted prompt.
- **Review** SHALL start an unsent reply naming the file -
  `While reviewing <file>, user made the following comments:` followed by a new
  line - and then offer **Submit Review** in place of the two.
- **Submit Review** SHALL send the reply with whatever the user added to it.
- Once a verdict is sent, none SHALL be offered until the file is read again.
  The agent revising the file is what reopens the question.

Each verdict SHALL reach the agent the way the user typing it would. For a
Panel-mode agent, Approve is delivered as a prompt, queued behind a running turn
as any other prompt is. Review appends the reply to the composer, keeping a
draft the user had begun, and focuses it. Submit Review is the composer's own
send. For a Terminal-mode agent, the text is typed at the prompt and a
submission is Escape then Return, with the Swift reference's delays between
them, so an agent CLI's autocomplete does not take the Return.

The messages are sent to the agent rather than shown as chrome, so they SHALL
NOT be localized.

#### Scenario: Approving a plan

- **WHEN** an idle Panel-mode agent has a markdown file open and the user
  activates Approve
- **THEN** the agent receives `approved let's do it` as a prompt, and the
  section offers no verdict until the file is read again

#### Scenario: Reviewing with comments

- **WHEN** the user activates Review, types a comment in the composer, and
  activates Submit Review
- **THEN** the agent receives the reply naming the file followed by the comment

### Requirement: Each section offers actions on its artifact

While a section is expanded, its header SHALL carry these controls, each with a
localized tooltip, between its title and its close control:

- The markdown section SHALL step the markdown font size down and up by one
  point, within 10 to 24 points, drawing a step disabled at its bound. The size
  SHALL be the persisted `markdown_font_size` setting, the same one the panel
  conversation's assistant messages use, so the two surfaces stay alike.
- The markdown section SHALL copy the file's contents to the clipboard,
  disabled until the first read lands.
- On macOS, the markdown section SHALL reveal the file in Finder.
- The mermaid section SHALL copy the diagram's source to the clipboard.

A collapsed section SHALL show none of these, only its chevron, title and close
control, as the Swift reference does.

Copy and reveal are additions of this port's own. Issue #535 asked for them, and
the Swift reference does not have them.

#### Scenario: Copying a file

- **WHEN** the user activates the markdown section's copy control
- **THEN** the clipboard holds the file's contents

#### Scenario: The font size stops at its bound

- **WHEN** the markdown font size is 24 points
- **THEN** the step-up control is disabled

### Requirement: Swift artifact actions this port leaves out

The following actions of the Swift reference SHALL be absent from this port, for
the reasons given, until a change adds them:

- **Comment on a selection.** `MarkdownPanelView` opens a comment popup on text
  selected in its web view while a review is active, and appends `- Re "<text>":
  <comment>` to the reply. The port renders markdown natively, with no text
  selection to anchor a popup to. A reviewer types comments into the reply that
  Review starts.
- **Diagram zoom.** `MermaidPanelView` zooms a rendered image from 0.25x to 4x.
  The port draws the diagram as laid-out native cards, not an image, so zoom
  would need a scale threaded through the layout and every card. The section
  scrolls instead.
- **Diagram theme picker.** `MermaidPanelView` picks among
  `BeautifulMermaid` themes. The port draws the diagram in the app's own theme
  colors, and has no second palette to choose.

#### Scenario: A diagram section has no zoom

- **WHEN** a mermaid section is expanded
- **THEN** its header offers copy and close, and no zoom or theme control

### Requirement: The panel can be shown and hidden from the menu bar and the keyboard

The View menu SHALL carry an **Artifacts** item, labelled through localization,
that toggles the selected agent's artifact panel. It dispatches the configurable
Toggle Artifacts shortcut's action, default ⌥⌘A, so the item shows the binding
beside its label and the item and the key do the same thing.

- While the selected agent's panel is shown, the toggle SHALL hide it, the same
  as close-all. What it hid is remembered like any close, so the next toggle
  brings back exactly what was hidden.
- While the panel is hidden and something closed can be reopened, the toggle
  SHALL reopen it, the same as the header's reopen control.
- The item SHALL be checked while the panel is shown.
- The item and the shortcut SHALL be disabled when the selected agent has no
  artifact shown and none to reopen, and when no agent is selected.
- Neither SHALL depend on the view mode. The panel stays beside the dashboard
  and pull requests, so it can be toggled while they are showing.

The header's reopen control SHALL remain. It is visible where the panel was,
and needs no knowledge of the menu or the key.

#### Scenario: Reopening from the header

- **WHEN** the user closes the selected agent's panel and activates the reopen
  control beside the agent's status
- **THEN** the panel is shown again with what it held

#### Scenario: Toggling from the View menu

- **WHEN** the selected agent's panel is shown and the user chooses View >
  Artifacts, and then chooses it again
- **THEN** the panel is hidden, and then shown again with the same sections

#### Scenario: Toggling from the keyboard

- **WHEN** the selected agent's panel is shown and the user presses ⌥⌘A twice
- **THEN** the panel is hidden, and then shown again with the same sections

#### Scenario: Nothing to toggle

- **WHEN** the selected agent has never had an artifact
- **THEN** View > Artifacts is disabled and ⌥⌘A does nothing

#### Scenario: The item shows the panel's state

- **WHEN** the selected agent's panel is shown
- **THEN** View > Artifacts is checked
