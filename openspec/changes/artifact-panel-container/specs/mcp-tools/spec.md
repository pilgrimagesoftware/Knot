# Spec Delta

## MODIFIED Requirements

### Requirement: display-markdown and view-mermaid

`display-markdown` SHALL require `agentId` and `filePath` and accept optional
`maximized`. `view-mermaid` SHALL require `agentId` and `source` and accept
optional `title`. Each SHALL update the target agent's panel state and return a
success indicator. `display-markdown` SHALL record a history of shown files,
most recent first.

`maximized` SHALL expand that agent's artifact panel, as `artifact-panel`
defines: the panel takes the whole content area rather than its set width.
Omitting it SHALL leave the panel at its set width.

This SHALL apply to every call that says something new about the panel, not only
the one that opens it: a call naming a different file, or the same file with a
different `maximized`, SHALL take effect. A file shown with `maximized` into an
already-open panel SHALL expand it, and a file shown without `maximized` into a
panel an earlier call maximized SHALL return it to its set width.

A call repeating both the file and the argument of the call before it SHALL
change nothing, so an agent re-showing a file it has just edited SHALL NOT
collapse a panel the user expanded by hand. Before this change the argument was
recorded and read by nothing, so an agent that asked for a maximized file got
the same panel as one that did not.

The two tools SHALL be independent: calling one SHALL NOT close or replace the
other's artifact. An agent that shows a file and then a diagram SHALL have both,
each in its own section of the panel.

#### Scenario: Show a markdown file

- **WHEN** `display-markdown` is called with a valid `filePath`
- **THEN** the agent's markdown panel targets that file and the file is
  prepended to its file history

#### Scenario: A maximized file opens an expanded panel

- **WHEN** `display-markdown` is called with `maximized` set
- **THEN** that agent's artifact panel is expanded to the whole content area

#### Scenario: A maximized file reaches an already-open panel

- **WHEN** `display-markdown` is called without `maximized`, and then called
  again for the same agent with it
- **THEN** the panel is expanded, rather than keeping the state the first call
  left it in

#### Scenario: Re-showing a file leaves the user's panel alone

- **WHEN** `display-markdown` is called without `maximized`, the user expands
  the panel by hand, and the agent calls it again for the same file with
  `maximized` still omitted
- **THEN** the panel is still expanded

#### Scenario: A diagram does not replace a file

- **WHEN** `view-mermaid` is called for an agent that already has a markdown
  file open
- **THEN** the agent has both the file and the diagram, and neither tool's
  result reports the other closed
