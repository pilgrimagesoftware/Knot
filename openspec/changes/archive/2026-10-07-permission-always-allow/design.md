# Design

## Decisions

**Resolve by kind, never by position.** Option order and wording are the
agent's presentation; `kind` is the protocol's meaning. Claude's adapter
proves position is unsafe: its first option is Always Allow, and on the
exit-plan-mode prompt it also changes the session's mode.

**A decision only ever takes its own kind.** Allow on a request offering only
Always Allow picks nothing. It does not escalate to the broader grant: the
user pressed the narrower key. Deny is the exception: with no `reject_once`
it takes `reject_always`, because refusing more than asked is the safe
direction.

**Keep the kindless path as it was.** An adapter that predates kinds gets the
old reading: Allow is the first option, and Deny is the first whose id or
name reads as a refusal. There is no Always Allow, since nothing says which
option would be one.

**Render the options, not a fixed pair.** `acp-panel-ui` already requires the
prompt to show "its available options as actionable controls". With kinds
known, every option is a button with the agent's own label. This also
covers prompts with several allow-always options, such as exit-plan-mode's
"auto-accept edits" and "bypass permissions". A pure `prompt_choices`
decides labels, emphasis and key hints, so the layout is tested without a
window.

**Lenient kind parsing.** `serde_json::from_value` on the option list failed
as a whole on any bad element. A custom deserializer turns an unknown or
non-string `kind` into `None` for that option, so a newer protocol revision
can't empty the prompt.

**Key.** `cmd-alt-shift-a`: Allow's chord with Option, the conventional
"more of the same" modifier on macOS, and unbound elsewhere in the keymap
(`cmd-alt-a` is Toggle Artifacts).
