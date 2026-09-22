## Purpose

Defines how Knot brings definitions in from tools the user already has -
coding-agent subagent definitions that become personas, and a Skwad
installation's workspaces, agents, personas and bench templates - and the
rules every such import obeys.

## ADDED Requirements

### Requirement: Import is additive and idempotent

An import SHALL only add records. It SHALL NOT delete, overwrite, rename or
reorder anything already in Knot, and SHALL NOT modify the source it read
from.

A record that Knot already holds SHALL be skipped rather than duplicated or
merged, so running the same import twice leaves the second run with nothing
to do. What counts as "already held" is defined per source below.

An import SHALL report what it did: how many records were added and how many
were skipped as already present.

#### Scenario: Importing the same source twice adds nothing the second time

- **WHEN** the user runs an import, then runs the same import again with the
  source unchanged
- **THEN** the second run adds nothing, skips every record as already
  present, and reports that

#### Scenario: Import leaves the source alone

- **WHEN** any import completes
- **THEN** the files or preferences it read are unchanged

#### Scenario: An import never removes what the user has

- **WHEN** an import runs in a Knot that already holds personas, agents and
  workspaces
- **THEN** every one of them is still present afterwards, with its name,
  contents and ordering unchanged

### Requirement: Partial failure does not abandon the import

A source that cannot be read, or an individual record that cannot be
understood, SHALL NOT fail the whole import. The readable records SHALL be
imported and the unreadable ones reported by name, so one malformed file
cannot block the other sixteen.

#### Scenario: One malformed definition among many

- **WHEN** a source holds ten definitions and one of them cannot be parsed
- **THEN** the other nine are imported and the failing one is named in the
  result

#### Scenario: A source that is not installed

- **WHEN** an import is offered for a tool that is not installed, or whose
  definitions directory does not exist
- **THEN** that source contributes nothing, reports nothing to import, and
  raises no error

### Requirement: The user chooses what to import

An import SHALL present what it found and let the user select which records
to bring in before anything is written. An import SHALL NOT write on the
strength of being opened.

#### Scenario: Reviewing before importing

- **WHEN** the user opens an import and the source holds records
- **THEN** those records are listed for selection, and nothing is added until
  the user confirms

#### Scenario: Cancelling changes nothing

- **WHEN** the user opens an import and closes it without confirming
- **THEN** no record is added

### Requirement: Subagent definition provider registry

The system SHALL resolve a subagent-definition provider by coding-agent tool.
The registry SHALL know the tools `claude`, `codex`, `opencode` and `gemini`.
A reader exists for `claude`. For a tool the registry does not know, and for
one it knows but has no reader for, subagent import SHALL be reported as
unsupported and SHALL be a no-op.

A provider SHALL read only; it SHALL NOT create, modify or delete anything in
the tool it reads from.

The import surface SHALL offer only the tools that have a reader. A tool
whose format is not yet confirmed SHALL be absent from it rather than listed
and empty: "nothing to import" is indistinguishable from "not built yet", and
a user cannot tell which one they are looking at.

Codex, OpenCode and Gemini have no reader because their subagent format has
not been read from a real installation. None of the three holds subagent
definitions on the machine this change was written on: Codex keeps
`config.toml` and a `skills/` directory with no agents, OpenCode's
`opencode.jsonc` holds only a `$schema` key, and `~/.gemini` holds config,
history and plugins but no agents. A reader written from an inferred format
fails silently, which is worse than no reader at all, so each remains
unimplemented until its format can be confirmed against a populated install.

#### Scenario: Unsupported tool

- **WHEN** subagent definitions are requested for a tool with no provider
- **THEN** support is reported false and no read is attempted

#### Scenario: A tool with no confirmed format is not offered

- **WHEN** the user opens the import surface
- **THEN** only tools with a reader are listed, and Codex, OpenCode and
  Gemini are absent rather than shown holding nothing

### Requirement: Claude subagent source and format

The Claude provider SHALL read `*.md` files from `~/.claude/agents` and, when
a folder is given, from that folder's `.claude/agents`. Each file is YAML
frontmatter delimited by `---` lines followed by a body.

The provider SHALL take the frontmatter's `name` as the definition's name and
the body - everything after the closing delimiter, trimmed - as its
instructions. Frontmatter keys Knot has no equivalent for, including
`description`, `color` and `model`, SHALL be ignored rather than folded into
the instructions.

A file with no frontmatter, no `name` key, or an empty body SHALL be reported
as unreadable rather than imported under a guessed name.

#### Scenario: A definition becomes a named persona

- **WHEN** `~/.claude/agents/architect-review.md` has frontmatter naming it
  `architect-reviewer` and a body beginning "You are an expert software
  architect"
- **THEN** the definition is offered as `architect-reviewer` with that body
  as its instructions, and its `description`, `color` and `model` are dropped

#### Scenario: Project definitions are found alongside user ones

- **WHEN** an import is run for a folder that has its own `.claude/agents`
- **THEN** definitions from both `~/.claude/agents` and that folder are
  offered

#### Scenario: A file without a name is not guessed at

- **WHEN** a `.md` file in the definitions directory has no frontmatter
  `name`
- **THEN** it is reported as unreadable, and is not imported under its file
  name

### Requirement: Imported subagent definitions become user personas

Importing a subagent definition SHALL create a persona with the definition's
name, its instructions, type `user`, and state `enabled`, per the `personas`
capability's persona model. It SHALL NOT create a `system` persona, which
would make it behave like one of Knot's shipped defaults under "Restore
Defaults".

A definition whose name matches an existing persona's name SHALL be skipped
as already present. Subagent definitions carry no identifier of their own, so
the name is what identity there is.

#### Scenario: An imported definition is editable like any persona

- **WHEN** a definition is imported
- **THEN** the resulting persona is a `user` persona that can be edited and
  deleted like one the user wrote

#### Scenario: A definition that was already imported

- **WHEN** a definition named `architect-reviewer` is imported and a persona
  by that name already exists
- **THEN** it is skipped, and the existing persona's instructions are left as
  they are

### Requirement: Skwad source and format

The Skwad provider SHALL read the preferences domain `com.kochava.skwad`,
whose collection values are JSON documents stored as data: `savedWorkspacesData`,
`savedAgentsData`, `personasData` and `benchAgentsData`. Their record shapes
are the ones Knot's own settings already read.

An absent preferences domain, or an absent key within it, SHALL be treated as
nothing to import rather than an error - Skwad may not be installed.

#### Scenario: Skwad is not installed

- **WHEN** the Skwad import is opened on a machine with no
  `com.kochava.skwad` preferences
- **THEN** it reports that there is nothing to import, and raises no error

#### Scenario: Skwad preferences are read, not written

- **WHEN** a Skwad import completes
- **THEN** Skwad's preferences are byte-for-byte unchanged, and a running
  Skwad is unaffected

### Requirement: Importing a Skwad workspace brings what it needs

Importing a workspace SHALL bring the agents it holds, and for those agents
any persona they reference and any bench template for their folder, so an
imported workspace is usable rather than a name with broken references.

An agent whose persona cannot be found in the source SHALL be imported
without one rather than skipped.

Records carry Skwad's own identifiers. A workspace, agent or persona whose id
Knot already holds SHALL be skipped as already present, which is what makes
re-importing safe.

#### Scenario: A workspace arrives with its agents

- **WHEN** the user imports a Skwad workspace holding three agents, two of
  which use a persona
- **THEN** the workspace, its three agents and that persona are added, and
  the agents are in the workspace in their original order

#### Scenario: An agent whose persona is missing

- **WHEN** an imported agent references a persona id that is not in Skwad's
  personas
- **THEN** the agent is imported with no persona, and the missing reference
  is reported

#### Scenario: Re-importing a workspace already brought across

- **WHEN** the user imports a workspace that was imported before
- **THEN** it is skipped as already present, and the agents in Knot's copy -
  including any added since - are untouched
