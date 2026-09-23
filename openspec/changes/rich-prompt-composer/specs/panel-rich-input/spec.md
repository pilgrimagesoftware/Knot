# Spec Delta

## Purpose

Defines how the panel composer presents the text being typed into it — slash
tokens, `@` references, markdown constructs and attachment references each
carry a visible treatment — without altering the characters the agent
receives.

## ADDED Requirements

### Requirement: Styling never changes the text

The composer's styling SHALL be presentation only. The characters in the
buffer, their order, and their offsets SHALL be exactly what the user typed
or pasted. The prompt delivered to the agent SHALL be byte-identical to the
buffer's text with no styling artefact added, no marker character removed,
and no whitespace normalised.

Selecting a styled range and copying it SHALL place the underlying characters
on the clipboard, including every markdown marker.

This diverges from no Swift behaviour: the Swift reference composer is plain
text throughout and has no styling of any kind.

#### Scenario: Styled markdown round-trips

- **WHEN** the user types `**ship it**` and sends the prompt
- **THEN** the agent receives `**ship it**`, asterisks included

#### Scenario: Copying a styled token

- **WHEN** the user selects a styled `/review` token and copies it
- **THEN** the clipboard holds `/review`

#### Scenario: Caret movement is unaffected

- **WHEN** the caret moves left across a styled range one position at a time
- **THEN** it visits every character of that range, because the range's
  characters are ordinary text

### Requirement: Slash and mention tokens are styled

A slash token or `@` reference in the buffer SHALL be drawn distinctly from
surrounding prose, so a token the lookup inserted is visibly a token. The
treatment SHALL apply to a token the user typed by hand exactly as it applies
to one the lookup inserted — the composer SHALL NOT track provenance.

A token SHALL be styled only where it is a token: a `/` mid-word, or an `@`
inside an email address, SHALL be left as prose.

#### Scenario: An inserted command is styled

- **WHEN** the lookup inserts `/review` into an empty buffer
- **THEN** that range is drawn with the token treatment

#### Scenario: A hand-typed token is styled the same

- **WHEN** the user types `/review` without opening the lookup
- **THEN** the range is drawn identically to an inserted one

#### Scenario: A slash inside a path is prose

- **WHEN** the buffer contains `crates/knot/src`
- **THEN** no part of it carries the token treatment

#### Scenario: An email address is prose

- **WHEN** the buffer contains `write to paul@example.com`
- **THEN** `@example.com` carries no mention treatment

### Requirement: A token being typed is styled as it grows

A token SHALL take its treatment from the first trigger character, before the
rest of the token exists, and SHALL keep it as the user types. The composer
SHALL NOT wait for the token to match a known command or an existing file.

A token that matches nothing SHALL still be styled as a token; whether it
resolves is the lookup's business, not the styling's.

#### Scenario: Styling appears on the trigger character

- **WHEN** the user types `/` at the start of a line
- **THEN** that character already carries the token treatment

#### Scenario: An unresolvable token is still a token

- **WHEN** the user types `/nosuchcommand`
- **THEN** the range carries the token treatment

### Requirement: Markdown constructs are styled in place

The composer SHALL style the markdown the user types: emphasis, strong
emphasis, inline code, fenced code blocks, headings, list markers, block
quotes and links. Each construct SHALL be visually distinguishable from
prose and from the other constructs.

Markers SHALL remain visible. The composer SHALL NOT hide, replace, collapse
or re-flow any character — an asterisk pair stays an asterisk pair, and a
fenced block keeps both fences.

A construct left unclosed SHALL be treated as prose from the point it fails
to close, so a lone `*` while typing does not italicise the rest of the
buffer.

#### Scenario: Strong emphasis

- **WHEN** the buffer contains `**ship it**`
- **THEN** the range is drawn with the strong treatment and both asterisk
  pairs are visible

#### Scenario: A fenced block

- **WHEN** the buffer contains a fenced block opened and closed with triple
  backticks
- **THEN** the block's contents carry the code treatment and both fences are
  visible

#### Scenario: An unclosed marker does not bleed

- **WHEN** the user types `a * b` with no closing asterisk
- **THEN** no part of the line carries the emphasis treatment

#### Scenario: An unclosed fence styles only what follows it

- **WHEN** the user types an opening fence and one line of text, with no
  closing fence
- **THEN** the line after the fence carries the code treatment and the text
  before the fence does not

### Requirement: Fenced code is not highlighted by language

A fenced block SHALL carry one code treatment regardless of its info string.
The composer SHALL NOT apply per-language syntax highlighting inside it.

A composer is for writing a request, not for reading a file, and a
language-aware highlighter is a cost paid on every keystroke for a few lines
of quoted code.

#### Scenario: An info string does not change the treatment

- **WHEN** the buffer holds one fenced block marked `rust` and another marked
  `python`
- **THEN** both blocks' contents are drawn with the same code treatment

### Requirement: An attached file has an in-buffer reference

When the user attaches context — through the add-context control, a drag from
the Finder, or a pasted image — the composer SHALL insert a reference to it at
the caret and SHALL style that reference as a chip: a bounded, visibly
self-contained run distinct from both prose and from a typed `@` mention.

The reference SHALL be ordinary text. Deleting it SHALL be possible with
ordinary editing, and doing so SHALL detach that context from the pending
message.

The thumbnail or file icon SHALL remain in the strip above the input. A chip
in the buffer is a text run, not an embedded image — this is a limitation of
the UI toolkit, recorded here so a reader does not mistake it for a design
preference.

#### Scenario: Pasting an image inserts a reference

- **WHEN** the user pastes a screenshot into the composer
- **THEN** a chip-styled reference to it appears at the caret and its
  thumbnail appears in the strip above the input

#### Scenario: Deleting the reference detaches the context

- **WHEN** the user selects a chip-styled reference and deletes it
- **THEN** that context is no longer attached to the pending message and its
  entry leaves the strip

#### Scenario: Attaching with an empty buffer

- **WHEN** the user attaches a file with nothing typed
- **THEN** the reference is the buffer's only content and the caret sits after
  it

### Requirement: Styling survives every way the composer changes

Styling SHALL be correct after any edit, not only after typing: paste,
multi-line paste, undo, redo, cut, drag-and-drop of text, and the lookup's
own insertion SHALL each leave every range styled as though the resulting text
had been typed.

Restoring a draft into the composer SHALL style it on arrival, with no edit
required to trigger the pass.

#### Scenario: Pasting a markdown document

- **WHEN** the user pastes several paragraphs containing headings, a fenced
  block and a list
- **THEN** every construct in the pasted text is styled

#### Scenario: Undo restores the prior styling

- **WHEN** the user deletes a fence's closing backticks and then undoes the
  deletion
- **THEN** the block is styled as a closed fenced block again

#### Scenario: A restored draft is styled

- **WHEN** a saved draft containing markdown is restored into the composer
- **THEN** it is styled without the user editing it

### Requirement: Styling keeps up with typing

The composer SHALL restyle only what an edit can have affected, not the whole
buffer. Typing SHALL NOT slow as the buffer grows: a character typed into a
long prompt SHALL appear as promptly as one typed into an empty composer.

The composer re-renders on every keystroke, so a full re-parse per character
is a defect, not a tuning question.

#### Scenario: Typing into a long prompt

- **WHEN** the user types into a composer holding several hundred lines
- **THEN** each character appears without perceptible delay

#### Scenario: An edit inside a fence restyles the fence, not the document

- **WHEN** the user types one character inside a fenced block in a long buffer
- **THEN** the work done is bounded by the construct being edited and the
  constructs an edit there can reopen or close

### Requirement: Styling reads correctly in both appearances

Every treatment SHALL be drawn from the active theme and SHALL remain legible
in the light and dark system appearances. A treatment SHALL NOT be the only
thing distinguishing two constructs when it relies on hue alone — weight,
slant, or a background SHALL carry the distinction as well.

Changing the system appearance SHALL restyle the composer without an edit.

#### Scenario: Appearance switches while composing

- **WHEN** the system appearance changes from light to dark with text in the
  composer
- **THEN** every styled range is redrawn in the new appearance's colours

### Requirement: The composer stays a composer

Adopting styling SHALL NOT turn the composer into a code editor. The composer
SHALL NOT show line numbers, indent guides, a gutter, fold controls, or a
minimap, and SHALL NOT auto-close brackets or quotes, re-indent on newline, or
expand tabs as an editor would.

Soft wrap SHALL remain on and the input SHALL keep growing with its content
between its collapsed and expanded bounds.

Every key the composer bound before SHALL keep its meaning: the send key, the
lookup's navigation keys, and the expand control.

#### Scenario: No editor chrome appears

- **WHEN** the composer is focused with several lines of text
- **THEN** no line numbers, gutter, indent guides or fold controls are drawn

#### Scenario: A bracket is not auto-closed

- **WHEN** the user types `(`
- **THEN** the buffer holds `(` alone

#### Scenario: Send still sends

- **WHEN** the user presses the configured send key with text in the composer
- **THEN** the prompt is sent, exactly as before this change

#### Scenario: The input still grows

- **WHEN** the user types past the composer's first line
- **THEN** the input grows as it did before this change, up to the same bound
