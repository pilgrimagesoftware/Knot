## Context

See proposal.md — Why. What shapes the approach:

- `conversation-history` already solves the same problem shape: one capability,
  a provider registry keyed by agent type, and a separate "source and format"
  requirement per tool, each written against the real on-disk layout. The
  subagent import is that pattern again, and should look like it rather than
  invent a second idiom.
- Knot's `SavedAgent`, `Workspace`, `Persona` and `BenchAgent` are ports of
  Skwad's Swift records, serialized camelCase. Skwad's JSON blobs deserialize
  into Knot's own types with no translation layer - the import is a read and a
  filter, not a migration.
- Only Claude Code's subagent format could be confirmed from disk. Codex,
  OpenCode and Gemini have no subagent definitions on this machine to read.
- The hand-off from `acp-only-agent-launch` records four bugs of one species:
  a format modelled from plausible inference rather than from the spec, failing
  silently. Three unconfirmed formats in one change is that trap, laid out.

## Goals / Non-Goals

**Goals:**

- One import surface, so a second source later is a section rather than a new
  place to look.
- Imports that a nervous user can run twice.
- Readers that cannot corrupt what they read.

**Non-Goals:**

- Syncing. This is a one-way copy at a moment the user chooses; an imported
  persona has no further relationship with the file it came from. Two-way sync
  is a different feature with a conflict model this does not need.
- Importing Skwad's scalar settings (fonts, autopilot, MCP port). They are the
  user's Knot preferences to set; silently adopting another app's is
  presumptuous and hard to notice.
- Exporting. Nothing here writes a foreign format.
- Importing Claude Code's *project folders* as agents. That was considered and
  set aside deliberately: 58 recorded folders on this machine, most of them
  one-off visits, is a list nobody wants to triage.

## Decisions

### A provider registry per tool, mirroring `conversation-history`

Each tool gets a provider behind one trait: does this tool have definitions,
and what are they. The registry resolves by tool name and reports unsupported
for anything else.

*Alternative considered:* one reader with per-tool branches. Rejected - it is
the same code with worse seams, and the existing capability already
established the registry idiom in this codebase. Matching it means a reader
who knows one knows the other.

### Only Claude's format is specified now; the other three are specified when read

Codex, OpenCode and Gemini all plausibly have subagent definitions, and this
machine has none of them to look at. Writing three requirements from plausible
inference is exactly the failure the hand-off documents four times over, and
its worst property is that it fails silently: a reader for a format that does
not exist finds nothing, which is indistinguishable from a user who has no
definitions.

So the registry lists all four tools and this change ships Claude's reader.
Each remaining provider is a task: confirm the format against a real
installation, write its requirement, then implement it. The work is committed;
only the guess is refused.

*Alternative considered:* ship all four now and correct them later. Rejected -
a wrong reader is worse than a missing one, because "nothing to import" looks
identical to working.

### Skip-if-present, keyed by id where there is one and by name where there is not

Skwad's records carry UUIDs, so identity is exact: an id Knot already holds is
already imported. Subagent definitions carry no identifier, so the persona name
is all the identity available, and a name collision is treated as
already-imported.

Both skip rather than merge. Merging needs a rule for whose text wins, and
every such rule surprises someone; skipping is explainable in one sentence and
never loses work.

*Alternative considered:* import duplicates with a suffix, the way Duplicate
Agent does. Rejected - running an import twice would then leave the user
deleting `architect-reviewer (copy)` seventeen times.

### Imports become `user` personas, never `system`

`personas` gives `system` personas different delete semantics and puts them
under "Restore Defaults". An imported persona is the user's own, from their own
file, and must not be resettable to something they never chose.

### Reading a plist means a new dependency

Skwad stores its collections as JSON inside a macOS preferences plist. Reading
it needs a plist parser, which the workspace does not have. `plist` is the
obvious crate.

Shelling out to `defaults read` was considered and rejected: it returns a
lossy text format that has to be re-parsed, it is macOS-only in a way a
library is not, and a subprocess whose output format is a UI convention is not
something to build a data import on.

The dependency is macOS-relevant only, but declared unconditionally so the
workspace goes on building on Linux for CI, where the import simply finds no
preferences.

### The import does not touch running agents

An imported workspace is added, not opened; imported agents are created in the
store like any other and start according to their activation mode. Nothing in
an import stops, starts or restarts anything.

## Risks / Trade-offs

- **An imported Skwad workspace may reference folders that no longer exist**
  → imported as-is. The agent shows its folder and fails to launch there, the
  same as any agent whose folder was deleted. Validating paths at import would
  invent a rule (skip? warn? fix?) that belongs to the user.
- **A subagent's system prompt may assume its host tool.** A Claude Code
  subagent prompt can mention tools Knot's agents do not have → imported
  verbatim anyway. Rewriting another tool's prompt is worse than importing it
  and letting the user edit; the persona editor is right there.
- **Name-keyed skipping hides an edited definition.** A user who edits
  `architect-reviewer.md` and re-imports gets nothing, because the name already
  exists → reported as skipped rather than silently ignored, which is what the
  result summary is for. Re-import-as-update can follow if it turns out to be
  wanted.
- **Three unimplemented providers in a shipped registry.** A user asked to
  import from Codex gets "nothing to import", which also means "not built yet"
  → the tab lists only providers that are implemented, so a tool with no
  reader is absent rather than falsely empty.

## Migration Plan

Nothing to migrate. The feature adds a tab and reads other applications'
files; removing it would leave imported records behind as ordinary Knot
records, indistinguishable from typed ones - which is the point.

## Open Questions

- Should a re-import offer to *update* a persona whose name matches but whose
  text has changed? Deferrable: skipping is specified and safe, the result
  already reports the skip, and turning it into an update changes no reader and
  no task here.
