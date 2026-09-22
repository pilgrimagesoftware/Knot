## 1. The import surface

The surface is a window of its own, not a settings pane: an import is an
action run once, not a preference kept. See `specs/import-ui/spec.md`.

- [x] 1.1 Add an `Import` window opened from File ▸ Import…, with a section
      per source and an empty-state message per section. Single-instance, like
      About and Settings. Verify that dispatching the menu action opens it,
      that choosing it again raises rather than duplicates, that it reopens
      after being closed, and that `SettingsTab::ALL` no longer names Import.
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

Deferred to the `import-subagents-from-more-tools` change. Each is blocked on
reading a real installation, and none of the three holds subagent definitions
on this machine. Writing a provider - or its spec requirement - from an
inferred format is the failure this change's design section refuses, so the
registry keeps them absent from the import surface until their format can be
confirmed.

- [x] 4.1 Record in `specs/data-import/spec.md` that Codex, OpenCode and
      Gemini have no reader, what was found on disk for each, and that the
      import surface lists only tools that have one. Move the three providers
      to a follow-up change rather than inferring their formats.

## 4a. Reaching the running application, and saying so

Found by testing the built app: an import wrote only to the settings
document, so the workspace manager never showed it and the next
store-to-settings write would have erased it; and the summary was rendered
past the bottom edge of an unscrolled window, so a successful import and a
failed one looked identical.

- [x] 4a.1 Add `AgentStore::adopt_saved`, taking records that already exist
      elsewhere while keeping their identifiers, skipping any the store
      already holds so the live copy and its session win. Verify with tests
      that identifiers survive, that an adopted workspace is still there in
      what the store writes back to settings, that a held record is not
      replaced, and that adopting twice adds nothing.
- [x] 4a.2 Put what an import wrote into the live store, so an open workspace
      manager shows it without a restart. Verify in the app with the manager
      open.
- [x] 4a.3 Make the outcome of every import visible: a pure summariser that
      is never silent, a failure drawn distinctly from a summary, and the
      panel pinned outside the scrolling region so a long list cannot push it
      off-screen. Verify with tests over the failed, did-nothing, added and
      unreadable cases.

## 5. Verification

- [x] 5.1 `make` passes clean.
- [x] 5.2 Confirm every import is additive: with a Knot holding personas,
      agents and workspaces, run both imports and verify nothing pre-existing
      changed name, contents or ordering.
- [x] 5.3 Confirm partial failure is tolerated: put one malformed `.md` among
      several valid ones and verify the valid ones import and the bad one is
      named.
