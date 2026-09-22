## 1. The import surface

- [x] 1.1 Add an `Import` tab to the settings window, with a section per
      source and an empty-state message per section. Verify with the existing
      settings-tab tests that the new tab appears in `SettingsTab::ALL`, has a
      distinct label, and that tab switching still preserves window state.
- [x] 1.2 Add the shared result type an import returns - added, skipped as
      already present, and unreadable records by name - and render it in a
      section after an import runs. Verify with a unit test that a result
      holding all three kinds renders each, including the names.

## 2. Personas from subagent definitions

- [x] 2.1 Define the provider trait (does this tool have definitions; what are
      they) and the registry that resolves one by tool name, reporting
      unsupported for anything else. Verify with a test that an unknown tool
      is unsupported and reads nothing.
- [x] 2.2 Implement the Claude provider against `~/.claude/agents/*.md` and a
      folder's `.claude/agents/*.md`: frontmatter `name`, body as
      instructions, other keys dropped. Verify with tests over fixture files
      covering a well-formed definition, one with no frontmatter, one with no
      `name`, and one with an empty body - the last three reported unreadable,
      not guessed at.
- [x] 2.3 Import selected definitions as `user` personas, skipping any whose
      name already exists. Verify with a test that a second import of the same
      definitions adds nothing and reports every one as skipped.
- [x] 2.4 Wire the personas section of the Import tab to the registry, listing
      only tools whose provider is implemented. Verify in the app against the
      real `~/.claude/agents` directory.

## 3. Workspaces from Skwad

- [x] 3.1 Add the `plist` dependency and a reader for the `com.kochava.skwad`
      preferences domain, decoding the JSON held in `savedWorkspacesData`,
      `savedAgentsData`, `personasData` and `benchAgentsData` into Knot's own
      record types. Verify with a test over a fixture plist, and one that an
      absent domain yields nothing rather than an error.
- [x] 3.2 Implement workspace import: bring the selected workspaces, the
      agents they hold, the personas those agents reference, and bench
      templates for their folders; skip any id Knot already holds; import an
      agent whose persona is missing without one. Verify with tests for each
      of those four behaviours.
- [x] 3.3 Wire the Skwad section of the Import tab. Verify in the app against
      the real Skwad preferences on a machine that has them, including that
      re-running the import adds nothing.
- [x] 3.4 Confirm the import does not write to Skwad: capture the plist's
      checksum before and after an import and verify it is unchanged.

## 4. The remaining providers

Each of these is blocked on reading a real installation. Do not write a
provider - or its spec requirement - from an inferred format; that is the
failure this change's design section refuses.

- [ ] 4.1 Codex: find where it keeps subagent definitions on a machine that
      has them, add the requirement to `specs/data-import/spec.md` describing
      that format, then implement the provider with fixture tests. If Codex
      has no such concept, record that in the spec instead and drop it from
      the registry.
- [ ] 4.2 OpenCode: same, starting from a populated `opencode.jsonc` or
      whatever its agent directory turns out to be.
- [ ] 4.3 Gemini: same.

## 5. Verification

- [x] 5.1 `make` passes clean.
- [x] 5.2 Confirm every import is additive: with a Knot holding personas,
      agents and workspaces, run both imports and verify nothing pre-existing
      changed name, contents or ordering.
- [x] 5.3 Confirm partial failure is tolerated: put one malformed `.md` among
      several valid ones and verify the valid ones import and the bad one is
      named.
