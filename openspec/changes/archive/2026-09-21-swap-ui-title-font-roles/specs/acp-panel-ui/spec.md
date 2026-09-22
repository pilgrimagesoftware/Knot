# Spec Delta

## ADDED Requirements

### Requirement: Rendered Markdown separates body from headers by font

Every Markdown surface — the panel's assistant messages and the Markdown pane
opened for an agent — SHALL draw body text in the configured UI font and
headers (`#` through `######`) in the configured title font.

Body text is everything but a header: paragraphs, list items, table cells,
block quotes, link text, and the text around inline code. Inline code and code
blocks SHALL keep the monospace family, as elsewhere in the panel.

A header SHALL keep the size and weight its level already gives it, so the two
faces are the change and the heading hierarchy is not.

Inline formatting inside a header — bold, italics, a link, inline code — SHALL
render as plain text in the title font. The renderer draws a header from its
text rather than its inline marks, and a header carrying a mark SHALL render as
the words themselves rather than as the Markdown source that marked them: `##
A **bold** word` renders as "A bold word", with no asterisks shown and no bold
run.

Changing either font in settings SHALL change the corresponding Markdown text
without reopening the surface.

#### Scenario: A response with a header and a paragraph

- **WHEN** an agent's response contains a `##` header followed by a paragraph
- **THEN** the header is drawn in the title font and the paragraph in the UI
  font

#### Scenario: Body constructs stay in the UI font

- **WHEN** a response contains a list, a table and a block quote
- **THEN** each is drawn in the UI font, and none of them in the title font

#### Scenario: Code keeps its own family

- **WHEN** a response contains a fenced code block and an inline code span
- **THEN** both are drawn in the monospace family, and the words around the
  inline span are drawn in the UI font

#### Scenario: A header keeps its level's size and weight

- **WHEN** a response contains an `#` header and a `###` header
- **THEN** both are drawn in the title font, at the sizes and weights their
  levels had before the two faces were split

#### Scenario: A header containing a mark

- **WHEN** a response contains the header `## A **bold** word`
- **THEN** the header line reads "A bold word" in the title font, with no
  asterisks and no bold run

#### Scenario: The Markdown pane follows the same split

- **WHEN** a Markdown file shown for an agent contains headers and paragraphs
- **THEN** its headers are drawn in the title font and its body in the UI font

#### Scenario: Changing the UI font redraws body text

- **WHEN** the user picks a new UI font family while a response with headers
  and paragraphs is on screen
- **THEN** the body text is redrawn in the new family and the headers are
  unchanged
