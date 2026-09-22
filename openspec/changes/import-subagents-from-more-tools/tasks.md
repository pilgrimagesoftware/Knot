## 1. Prerequisite

Each tool's format is *documented*; none is *confirmed*. All three are
installed on this machine and hold zero definitions - no `agents/` directory
exists for any of them. The documented formats below are leads to check a
real installation against, not a substitute for reading one. Writing a reader
from them is the inferred-format failure the parent change refused.

All three tools do have a subagent concept, so none is dropped from the
registry on those grounds.

- [ ] 1.1 For each tool, find a machine with a populated installation - one
      that actually holds subagent definitions. A tool with no such
      installation available stays unimplemented. Record for each tool which
      machine was read and when.

## 2. Codex (#302)

Documented: `~/.codex/agents/*.toml` (user) and `.codex/agents/*.toml`
(project). TOML, not Markdown. Required `name`, `description`,
`developer_instructions`; optional `model`, `model_reasoning_effort`,
`sandbox_mode`, `nickname_candidates`, `[mcp_servers.*]`, `skills.config`.
Instructions are a `developer_instructions` string, not a file body.
https://developers.openai.com/codex/subagents

- [ ] 2.1 Confirm the above against a populated installation: the file
      location, that `developer_instructions` carries what Knot calls
      instructions, and every way a definition can be malformed.
- [ ] 2.2 Add the Codex source-and-format requirement to
      `specs/data-import/spec.md`, describing the confirmed format and what
      is reported unreadable.
- [ ] 2.3 Implement the provider behind `SubagentProvider`. Verify with
      fixture tests over a well-formed definition and one fixture per failure
      mode named in 2.1, and that a missing directory yields an empty scan
      rather than an error. This is the first TOML definition format, so the
      trait's Markdown assumptions may need widening.

## 3. OpenCode (#303)

Documented: the loader globs `{agent,agents}/**/*.md` under
`~/.config/opencode/` and a project `.opencode/` - both spellings, and
**recursively**. The agent name is the path *below* that directory, so
`agents/team/reviewer.md` defines `team/reviewer`, not `reviewer`. Markdown
with YAML frontmatter; the body becomes the system prompt. `mode: subagent`
distinguishes a subagent from a primary agent, and agents may also be defined
inline under the config's `agent` (v1) / `agents` (v2) key.
https://opencode.ai/docs/agents/ and
`packages/opencode/src/config/agent.ts`

- [ ] 3.1 Confirm the above against a populated installation. Two points the
      documentation contradicts itself on and the installation must settle:
      whether `mode` defaults to `all` or to `primary` when omitted, and
      therefore whether a definition with no `mode` should be offered as a
      persona at all. Also decide whether inline config agents are in scope
      or only the Markdown files.
- [ ] 3.2 Add the OpenCode requirement to `specs/data-import/spec.md`,
      including how a nested path becomes a name.
- [ ] 3.3 Implement the provider with fixture tests, as in 2.3, covering a
      nested definition and both directory spellings.

## 4. Gemini (#304)

Documented: `~/.gemini/agents/*.md` and `.gemini/agents/*.md`. Markdown with
YAML frontmatter; the body becomes the system prompt. Required `name`,
`description`; optional `kind`, `tools`, `mcpServers`, `model`,
`temperature`, `max_turns`, `timeout_mins`.
https://github.com/google-gemini/gemini-cli/blob/main/docs/core/subagents.md

`kind: remote` is a different record entirely: it carries no system prompt,
pointing at an A2A agent-card URL instead, and one file may hold a list of
several. Importing one as a persona would produce a persona with no
instructions.

- [ ] 4.1 Confirm the above against a populated installation, including what
      a `kind: remote` file looks like in practice.
- [ ] 4.2 Add the Gemini requirement to `specs/data-import/spec.md`, stating
      that only `kind: local` (or absent) definitions are offered and that a
      remote one is skipped rather than reported unreadable - it is a
      well-formed record of a kind Knot has no equivalent for.
- [ ] 4.3 Implement the provider with fixture tests, as in 2.3, including a
      remote definition and a multi-record remote file.

## 5. Verification

- [ ] 5.1 Narrow the registry requirement in `specs/data-import/spec.md` to
      name only the tools still without a reader, removing any that landed.
- [ ] 5.2 Verify each new tool appears in the Import tab without a UI change,
      and that importing from it obeys the additive and idempotent guarantees
      the `data-import` capability already states.
- [ ] 5.3 `make` passes clean.
