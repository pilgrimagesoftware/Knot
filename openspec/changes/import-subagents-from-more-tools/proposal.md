## Why

`import-from-other-tools` shipped the subagent-definition provider registry
with one reader behind it. Claude Code's format was confirmed against real
files on disk; Codex, OpenCode and Gemini were not, because no machine
available to that change held subagent definitions for any of them:

- Codex keeps `~/.codex/config.toml` and a `skills/` directory, with no
  agents.
- OpenCode's `opencode.jsonc` holds only a `$schema` key.
- `~/.gemini/` holds config, history and plugins, with no agents.

That change refused to write three readers from inferred formats, because a
reader for a format that does not exist finds nothing - which is
indistinguishable from a user who has no definitions. The registry therefore
knows all four tools while the Import tab lists only the one with a reader.

This change finishes the other three, one at a time, each against a populated
installation.

## What Changes

One provider per issue - Codex #302, OpenCode #303, Gemini #304 - in any
order, as an installation of each becomes available. For each:

- Read a real installation that holds subagent definitions and confirm where
  the tool keeps them and what shape they are in.
- Write that tool's source-and-format requirement in
  `specs/data-import/spec.md`, describing the confirmed format.
- Implement the provider behind the existing `SubagentProvider` trait, with
  fixture tests covering a well-formed definition and each way one can fail
  to be read.
- The tool then appears in the Import tab automatically, since the tab lists
  whatever `SubagentTool::implemented()` returns.

All three tools do have a subagent concept - each documents one, and the
documented location and format for each is recorded in `tasks.md` as a lead
to check a populated installation against. None is dropped from the registry
on the grounds of having no such concept. What is missing is a confirmed
format, not the feature.

Documentation is not confirmation. Two of the three already show why: the
OpenCode loader accepts both `agent/` and `agents/` and derives the name from
the nested path rather than the filename, and Gemini's `kind: remote`
definitions carry no system prompt at all, so a reader that imported them
would produce personas with no instructions. Both are the kind of detail a
real installation settles and a documentation page does not.

## Capabilities

### Modified Capabilities
- `data-import`: adds a source-and-format requirement per tool whose format
  has been confirmed, and narrows the registry requirement as each reader
  lands.

## Impact

- `knot-core`: one provider module per tool under `import/subagents/`, each
  behind the trait the registry already defines.
- `crates/knot`: none. The Import tab already renders whatever the registry
  reports as implemented.
- Blocked on access to a populated installation of each tool. Nothing here
  can be implemented from documentation alone without reintroducing the
  inferred-format failure the parent change refused.
- Applies after `import-from-other-tools` is archived. Its delta MODIFIES the
  registry requirement, which only exists as a spec once the parent change
  has created the `data-import` capability.
