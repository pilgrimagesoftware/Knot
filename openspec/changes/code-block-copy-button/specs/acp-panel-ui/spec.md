# Spec Delta

## ADDED Requirements

### Requirement: A rendered code block carries a copy control

Every code block drawn on a Markdown surface — the panel's assistant messages
and the Markdown pane opened for an agent — SHALL carry a copy control in the
block's upper-right corner, inside the block's own background.

Activating it SHALL place that block's code on the system clipboard and SHALL
confirm with a transient notification, in the same style the panel's existing
copy actions use. The text placed on the clipboard SHALL be the code as
written inside the block: no fence markers, no language tag, and nothing from
the surrounding prose.

The control SHALL be present whenever the block is on screen, not only while
the pointer is over it, and SHALL carry a localized tooltip naming what it
copies. Its tooltip and its confirmation SHALL both be resolved through
localization rather than carried as literal English in the renderer.

Inline code spans SHALL NOT carry a copy control: only block-level code does.

The control SHALL NOT change how the code itself is drawn — same monospace
family, same block background, same wrapping — and SHALL NOT replace or
disturb the response action bar's existing "copy response", which continues to
copy the whole response.

This diverges from the Swift app, which renders a code block as plain
preformatted text in a web view with no copy affordance of any kind.

#### Scenario: Copying one block's code

- **WHEN** a response contains a fenced code block and the user activates that
  block's copy control
- **THEN** the system clipboard holds exactly the lines inside the fence, with
  neither the fence markers nor the language tag, and a confirmation
  notification is shown

#### Scenario: Several code blocks in one response

- **WHEN** a response contains three code blocks and the user activates the
  second block's copy control
- **THEN** the clipboard holds the second block's code alone, and the other two
  blocks' contents are not included

#### Scenario: The Markdown pane carries the same control

- **WHEN** a Markdown file shown for an agent contains a code block
- **THEN** that block carries the same copy control in the same corner, with
  the same copied text, as a code block in an assistant message

#### Scenario: Inline code is left alone

- **WHEN** a response contains an inline code span in the middle of a sentence
- **THEN** no copy control is drawn for that span

#### Scenario: The control does not wait for a hover

- **WHEN** a code block is on screen and the pointer is elsewhere
- **THEN** the block's copy control is still drawn in its upper-right corner

#### Scenario: Copying a response is unaffected

- **WHEN** a response containing a code block is finished and the user
  activates "copy response" on its action bar
- **THEN** the clipboard holds the full response text, code block and prose
  alike, exactly as it did before code blocks carried their own control
