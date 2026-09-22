# Proposal

## Why

The panel prompt box knows only text. Reaching a named command or a skill
means typing the whole token from memory into a plain textarea with no
feedback, and nothing in the app tells the user what is available. The agents
Knot runs are already command-and-skill driven; the panel that talks to them
should surface that vocabulary as it is typed instead of hiding it.

## What Changes

- When the user types a leading `/` in the panel prompt input, a **lookup
  popup** appears above the input listing the commands and skills Knot knows.
- The list filters as the user types after the `/`; Up/Down move the
  selection, Enter or Tab inserts the selected entry, Esc dismisses.
- Inserting an entry **replaces the partially typed token** with the entry's
  full token; the rest of the buffer is untouched.
- The entries come from a **registry**: Knot's built-in panel commands plus
  the skills available to the selected agent.
- The lookup **completes text only** - it does not execute commands. The
  completed prompt is sent to the agent like any other.
- The popup dismisses when the token disappears, nothing matches, focus
  leaves the input, or the user presses Esc.

## Capabilities

### New Capabilities
- `panel-slash-commands`: the slash-triggered lookup over a registry of
  commands and skills for the panel prompt input, including what triggers it,
  how it filters and is navigated, and what inserting an entry does.

### Modified Capabilities
(none - the lookup is a new concern beside the input-area requirements in
`acp-panel-ui`, which keeps owning the input area's existing controls.)

## Impact

- `crates/knot` workspace window: the panel input area gains the popup, and a
  new command/skill registry module supplies the entries. Both are read-only
  sources; nothing executes.
- The registry deals in text (a token, a description), so it has no
  dependency on any agent's internals.
- User-facing strings go through `knot_core::l10n::t`.