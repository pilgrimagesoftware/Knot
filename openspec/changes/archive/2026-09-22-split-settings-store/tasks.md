# Tasks

## 1. Constants and paths

- [x] 1.1 Replace `SETTINGS_FILE` in `knot-core/src/consts.rs` with one constant per document (`PREFERENCES_FILE`, `AGENTS_FILE`, `WORKSPACES_FILE`, `PERSONAS_FILE`, `BENCH_FILE`, `RECENT_REPOS_FILE`), plus `LEGACY_SETTINGS_FILE` and the migrated suffix, and rename the temp extension to cover every document; verify `cargo build --workspace` names the remaining `SETTINGS_FILE` uses as the only errors.
- [x] 1.2 Add `settings/store/paths.rs` with a `StorePaths` type holding the preferences file path and the data directory, deriving each collection document's path, and building from `ProjectDirs::preference_dir()` and `ProjectDirs::config_dir()`; verify a unit test asserts the six derived paths from a fixed root and that the preferences path is not under the data directory.
- [x] 1.3 Replace `Settings::store_path` with `paths: Option<StorePaths>` and `Settings::with_store_path` with `Settings::with_store_root(dir)`; verify `cargo build -p knot-core` succeeds.

## 2. Per-document read and write

- [x] 2.1 Add `settings/store/documents.rs` with the atomic write helper (temp file beside the target, rename into place, remove the temp file on a failed rename) parameterized by path and bytes; verify a unit test asserts no temp file remains after a successful write.
- [x] 2.2 Add a tolerant collection reader in `documents.rs` that decodes a bare JSON array per record, dropping records that fail, and yields an empty collection for a missing, unreadable, non-JSON or non-array document; verify unit tests cover each of those four cases plus a mixed good/bad array.
- [x] 2.3 Mark the five collection fields on `Settings` `#[serde(skip)]` and drop the now-unused `de_tolerant_vec` container attributes; verify a round-trip test shows the serialized preferences document holds no `savedAgents`, `savedWorkspaces`, `personas`, `benchAgents` or `recentRepos` key.
- [x] 2.4 Rewrite `Settings::persist` to write all six documents and add private per-document writers; verify a test that persists a full store finds all six files on disk with the expected contents.
- [x] 2.5 Point each mutating helper at the writer for the document it changes (scalar setters at preferences, persona helpers at personas, and likewise for agents, workspaces, bench and recent repos); verify a test that records the other five documents' bytes, mutates one collection, and asserts those five are unchanged.
- [x] 2.6 Rewrite `Settings::load`/`load_from` to read the preferences document and each collection document independently, keeping the font-role migration, the `"SF Mono"` replacement and the sidebar clamp on the preferences path; verify tests that corrupt one collection document and, separately, the preferences document, and assert the rest loads intact.

## 3. Legacy migration

- [x] 3.1 Add `settings/store/legacy.rs` that detects `settings.json` in the data directory, lifts the five collection keys out of the parsed `Value`, decodes the remainder as preferences, and returns the assembled `Settings`; verify a unit test over a legacy document carrying every scalar and every collection reads each value back.
- [x] 3.2 Apply the "new documents win" rule so an existing collection document overrides the legacy document's value for that collection; verify a test where a legacy document and a written `personas.json` disagree loads the written file's personas.
- [x] 3.3 Write all six documents and then rename `settings.json` to `settings.json.migrated`, only after every write succeeds; verify a test asserts the six files exist, `settings.json` is gone and `settings.json.migrated` holds the original bytes.
- [x] 3.4 Treat an unreadable or undecodable legacy document as absent and leave it under its own name; verify a test with unparseable content loads defaults and finds `settings.json` still present.
- [x] 3.5 Wire migration into `Settings::load` and `load_from_root` so it runs only when the legacy document is present; verify a test that loads twice asserts no migration runs the second time and no `settings.json` is recreated.
- [x] 3.6 Confirm the font-role migration survives the move; verify a test that migrates a legacy document with no `settingsVersion` marker and both fonts customized finds the swapped values and the current marker in the written preferences document.
- [x] 3.7 Verify migration is crash-safe by loading against a data directory that already holds some new documents alongside the legacy one and asserting the result matches a clean migration.

## 4. Call sites and tests

- [x] 4.1 Update the `Settings::with_store_path` call sites in `knot-core/tests/settings.rs`, `knot-core/src/settings/store/tests.rs`, `knot/src/tests/workspace_dialog.rs` and `knot-mcp-tools/src/lib.rs` to `with_store_root`; verify `cargo test --workspace` compiles.
- [x] 4.2 Move the migration tests into `settings/store/legacy/tests.rs` and keep the store tests in `settings/store/tests.rs`; verify `make size-check` passes with no file over 700 lines.

## 5. Documentation and gate

- [x] 5.1 Update the module docs on `settings/store.rs` to describe the document set, the two directories and the legacy migration, linking the contract at `openspec/specs/settings-persistence/spec.md`; verify the doc comment names each of the six documents.
- [x] 5.2 Update the workspace layout notes in `CLAUDE.md`/`AGENTS.md` if they name `settings.json`; verify a repo-wide grep for `settings.json` returns only migration code, its tests and this change's artifacts.
- [x] 5.3 Run the full gate; verify `make` passes (`fmt-check`, `size-check`, `lint`, `test`, `build`).
- [x] 5.4 Verify the migration against a real installation: copy an existing `settings.json` into a scratch data directory, load through `with_store_root`, and confirm every agent, workspace and persona reads back and the preferences document lands in the preferences directory.
