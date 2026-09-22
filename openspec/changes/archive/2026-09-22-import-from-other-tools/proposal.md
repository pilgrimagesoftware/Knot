## Why

Knot asks the user to type in everything it knows. Personas are written from
scratch in the settings window, and a workspace of agents is assembled folder
by folder - even though the user already has both, elsewhere.

Two sources are sitting on disk right now:

- **Coding-agent subagent definitions.** Claude Code keeps 17 of them in
  `~/.claude/agents/` on this machine: a name, a description, a model, and a
  system prompt. The system prompt is exactly what a Knot persona is, written
  and refined already.
- **Skwad, Knot's own predecessor.** Its workspaces, agents, personas and
  bench templates are in `~/Library/Preferences/com.kochava.skwad.plist`, in
  the same record shapes Knot's settings already deserialize - Knot's records
  are a port of them. Anyone moving from Skwad to Knot currently rebuilds
  their whole setup by hand for no reason.

## What Changes

- Add an **Import window**, opened from File ▸ Import…, the single place both
  imports live. It is a window rather than a settings pane: an import is an
  action the user runs once, not a preference Knot keeps.
- **Import personas from subagent definitions.** A provider per coding-agent
  tool finds that tool's subagent definitions and offers them; the user picks
  which to import, and each becomes a `user` persona.
  - **Claude Code's provider ships with this change.** Its format is
    confirmed against real files.
  - **Codex, OpenCode and Gemini are deferred to a follow-up change.**
    Nothing on this machine has subagent definitions for any of the three:
    Codex has `~/.codex/config.toml` and a `skills/` directory with no
    agents, OpenCode's `opencode.jsonc` holds only a `$schema` key, and
    `~/.gemini/` has config, history and plugins but no agents. Each
    provider's requirement is written once its format has been read from a
    real installation, not inferred - inferring a format is the mistake that
    produced four silent failures on the last branch. The registry knows all
    four tools; the import tab lists only the ones with a reader, so an
    unimplemented tool is absent rather than falsely empty.
- **Import workspaces from Skwad.** Read Skwad's preferences, list its
  workspaces, and import the chosen ones along with the agents they hold, the
  personas those agents reference, and the bench templates.
- **Import only ever adds.** It never deletes, overwrites, or reorders what
  the user already has. A record that is already present is skipped, so
  running an import twice changes nothing the second time.

## Capabilities

### New Capabilities
- `data-import`: bringing existing definitions into Knot from tools the user
  already has - which sources are read, what each contributes, and the rules
  every import obeys (additive only, idempotent, partial failure tolerated).

### New Capabilities
- `import-ui`: the Import window - where an import is run from, what it shows
  before it writes anything, and what it reports afterwards.

## Impact

- `knot-core`: a persona-import reader per tool and a Skwad preferences
  reader; both produce records the existing settings store already holds.
  Reading a `.plist` needs a plist parser - the first new third-party
  dependency this port has taken on for a feature.
- `crates/knot`: the Import window, its two sections, and the selection list
  each import presents before it does anything. The two control constructors
  it shares with the settings window (`group`, `icon_button`) move out of
  `SettingsWindow` into `crate::controls`, which the agent editor and
  workspace manager were already reaching for by name.
- No change to `knot-git`, `knot-discovery`, the MCP server, or how agents
  launch. Nothing here touches a running agent.
- Skwad is read-only throughout. Import never writes to its preferences, so a
  user can keep using it while they move across.
