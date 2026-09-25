# Tasks

## 1. Data model (`knot-core`)

- [x] 1.1 Add the `Prompt` record (id, name, text) and the `StartupPrompt` enum (`Library(Uuid)`, `Custom(String)`) with serde tagging; verify a round-trip test for each variant and that a blank-name or blank-text prompt is refused by the constructor
- [x] 1.2 Add `#[serde(default)] startup_prompt: Option<StartupPrompt>` to `SavedAgent` and `BenchAgent`, storing blank custom text as `None`; verify a legacy agent and a legacy bench entry decode with `None`, and a custom prompt round-trips through the agents document
- [x] 1.3 Add `PROMPTS_FILE` to `consts.rs`, the `prompts` collection, its path and `persist_prompts`; verify a store with no prompts document loads an empty library and writes none, and that seven collection documents exist after persisting a full store (update the six-document test)
- [x] 1.4 Add library add/update/remove that persist before returning, keeping insertion order; verify removing a referenced prompt leaves the agents document's bytes unchanged
- [x] 1.5 Add `remove_bench_agent(id)` if not already present; verify it removes only that entry and persists the bench document
- [x] 1.6 Verify the bench keeps a library reference as a reference: save an agent referencing P to the bench, reload, and assert the entry holds `Library(P)`, not P's text

## 2. Prompt variables (`knot-agent-launch`)

- [x] 2.1 Add `PromptVariable` with `Display`/`FromStr` over the eight dotted names; verify every name round-trips and that a case-changed name (`Folder`) does not parse
- [x] 2.2 Add `PromptContext` and the single-scan `expand`; verify unit tests for each variable, whitespace inside braces, an unknown name left verbatim, `\{{` escaping with the backslash dropped, an unclosed `{{` left alone, and a value containing `{{date}}` not re-expanded
- [x] 2.3 Add `unknown_variables(text)` sharing the scanner; verify it reports `foldr` for `{{foldr}}`, nothing for escaped or known names, and each unknown name once
- [x] 2.4 Add a context builder that reads `branch` (short `HEAD` hash when detached, empty outside a repository) and `date` (local, UTC fallback), enabling `time`'s `local-offset` feature; verify against a temp repository on a branch, a detached HEAD, and a non-repository folder, with git config isolated

## 3. Resolution and launch (`knot-agent-launch`, `knot-activity`, `knot-terminal`)

- [x] 3.1 Add `resolve_startup_prompt(&Option<StartupPrompt>, &[Prompt]) -> Option<String>` returning raw text, and a one-line form applied after expansion; verify unit tests for none, a live reference, a dangling reference (`None`), custom text, and line breaks (including ones a value contributed) becoming single spaces
- [ ] 3.2 Add a `startup_prompt` one-shot to the activity state that fires `Effect::InjectStartup` at the first Idle after `InjectRegistration`; verify it never fires without a prior registration injection and fires exactly once
- [ ] 3.3 In `knot-terminal`, build the context at launch, expand and flatten, and set the startup prompt only when the registration prompt is set; type it followed by Return on `InjectStartup`; verify with a tracker test that a resumed launch sets neither

## 4. ACP delivery (`knot`)

- [ ] 4.1 Add `PromptOrigin::Startup` with its own `panel.queued_startup_prompt` label; verify the queue-row accessible name differs from the user and inbox-nudge names
- [ ] 4.2 In `ensure_panel_session`, resolve the reference to raw text when a registration prompt is built and pass it into the connect task, which reads `branch`, expands, and stores the result on the session handle before publishing `Ready`; verify a resume (`session_to_load` set) carries none
- [ ] 4.3 Mark `turn_active` in `connect_into` when the registration message is recorded, before publishing `Ready`; verify with the stalled fake adapter that the startup prompt stays queued while the first turn is unanswered, and is delivered after it ends
- [ ] 4.4 Take the handle's startup prompt into the queue in `drain_panel_prompt` the first time the slot is `Ready`; verify it is queued exactly once and for an unselected agent too
- [ ] 4.5 Verify Register Agent sends only the registration prompt and Restart with New Conversation queues the startup prompt again

## 5. Agent store and benching (`knot-agents`, `knot`)

- [x] 5.1 Carry `startup_prompt` through `CreateOptions` and `deploy_bench`; verify a deployed entry's agent holds the entry's startup prompt in the same form, and the entry stays on the bench after two deployments
- [ ] 5.2 Extract `bench_entry_for(&Agent)` from Save to Bench, including the startup prompt, and use it from Save to Bench; verify the existing Save to Bench tests still pass
- [ ] 5.3 Implement benching: write the entry through `write_persisting`, then remove the agent (and its companions) only on success; verify a failed write leaves the agent in place, and an owner with a companion yields one bench entry and two removals
- [x] 5.4 Exclude the startup prompt from launch-affecting edits; verify changing only the startup prompt does not recreate the session
- [ ] 5.5 Add `WorkspaceWindow::deploy_bench_entry(entry, cx)`: deploy into the sidebar's workspace and select; on a missing folder remove the entry and post a notification naming it; verify both paths with a window test

## 6. Sidebar menus (`knot`)

- [ ] 6.1 Add `AgentMenuEntry::BenchAgent` after Save to Bench, shown only for non-companions; verify the agents-menu and context-menu ordering tests and that a companion hides both bench items
- [ ] 6.2 Confirm before benching, with a body that says the conversation is not kept and, for an owner, that its companions close; verify cancel leaves the agent and bench unchanged
- [ ] 6.3 Add the New from Bench submenu to the sidebar background menu after New Agent, built from the settings surface's bench at menu-build time, disabled when the bench is empty, calling `deploy_bench_entry`; verify the background-menu ordering and enablement tests

## 7. Bench popover - Swift dropdown port (`knot`)

- [ ] 7.1 Split `new_agent_button` into the existing main button and a chevron that toggles a gpui-kit `Popover`; verify the main part still opens the agent editor with no popover, and the compact layout keeps the chevron with a tooltip
- [ ] 7.2 Add `workspace_window/bench_popover.rs`: Create New Agent row, divider, BENCH header, entries (avatar, name, folder name, one line each) in bench order, scrolling past 300px, and the empty-bench hint; verify both the populated and empty renders
- [ ] 7.3 Read the bench from the settings global when the popover opens; verify an entry saved from another window after the window last drew is listed
- [ ] 7.4 Clicking an entry calls `deploy_bench_entry` and closes the popover; Create New Agent closes it and opens the editor; Escape and an outside click close it without acting; verify each
- [ ] 7.5 Show the remove control on hover or focus, confirm naming the entry, remove via `remove_bench_agent`, and re-read so the popover stays open on the updated list; verify removing never deploys and cancel keeps the entry
- [ ] 7.6 Give the chevron, Create New Agent, each entry and each remove control a keyboard focus stop and an accessible name; verify by tabbing through the popover in a window test

## 8. Agent editor (`knot`)

- [ ] 8.1 Add the Startup Prompt control (None, library prompts by name, Custom with a multi-line field) for non-`shell` agents, with the caption; verify a new agent defaults to None and Custom reveals the text field
- [ ] 8.2 Add the variable list and unknown-variable warnings to the custom field, without blocking submit; verify `{{brnach}}` is flagged and the form still submits
- [ ] 8.3 Show a dangling reference as a missing prompt and keep it when unchanged; clear the startup prompt when the type becomes `shell`; verify both on submit

## 9. Settings window (`knot`)

- [ ] 9.1 Add `SettingsTab::Prompts` and `SettingsTab::Bench` after Personas; verify the tab list and that switching tabs preserves window state
- [ ] 9.2 Implement the Prompts tab and its editor after `persona_editor.rs`: list with preview, add, edit, cancel, save disabled on a blank field; verify each Prompts-tab scenario
- [ ] 9.3 Add the editor's variable list (inserting at the caret) and unknown-variable warnings; verify choosing `folder.name` inserts `{{folder.name}}` at the caret and `{{foldr}}` warns without disabling save
- [ ] 9.4 Delete immediately when unreferenced, otherwise confirm with counts of referencing agents and bench entries; verify both paths, and that cancel keeps the prompt
- [ ] 9.5 Implement the Bench tab: list, edit name and startup prompt only, remove with confirmation (the helper shared with the popover), empty-state message; verify each Bench-tab scenario

## 10. Slash lookup (`knot`)

- [ ] 10.1 Add a library-prompt source to the lookup registry, read from the settings global at lookup time, with entries marked as prompts; verify `/gate` lists "Run the gate" and a prompt added while a composer is open appears on the next lookup
- [ ] 10.2 On inserting a prompt, capture the token range and buffer revision and expand on `spawn_blocking` against the selected agent's context; verify the composer is not blocked
- [ ] 10.3 Deliver the expansion through a slot in `repaint_poll_tick`'s `if` chain, applied only when the revision still matches; verify the `note: /gat` and `Review {{folder.name}}` scenarios, and that editing the token first drops the expansion

## 11. Conventions and close-out

- [ ] 11.1 Add every user-facing string to `crates/knot-core/locales/en.yml` (`settings.prompts.*`, `settings.bench.*`, `prompt_variables.*`, `menu.agent.bench*`, `menu.sidebar.new_from_bench`, `sidebar.bench_popover.*`, `agent_editor.startup_prompt.*`, `panel.queued_startup_prompt`); touch `knot-core` first, then verify with key-resolution tests
- [ ] 11.2 Run `make` and verify `fmt-check`, `size-check`, `lint`, `test` and `build` all pass, with no crate-wide `allow` and no implementation in a `mod.rs`
- [ ] 11.3 Tests that persist settings use `with_store_root`; verify no test writes to the real settings directory
- [ ] 11.4 Manual verification in the running app (**needs a display**): create a library prompt using `{{agent.name}}` and `{{branch}}`, set it as an ACP agent's startup prompt, restart with a new conversation and see registration then the expanded prompt as two turns; edit the queued prompt before delivery; bench the agent, redeploy it from the New Agent chevron's popover and again from New from Bench, and see the prompt sent each time; remove an entry from the popover; expand the prompt from `/` in a composer
