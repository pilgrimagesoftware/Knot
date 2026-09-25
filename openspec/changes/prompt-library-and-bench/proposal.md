# Proposal

## Why

Every agent starts on the same initialization prompt: the knot instructions,
its persona, and "Register with the knot". To give an agent its actual work
the user has to type it after registration, every launch, and retype the same
standing instructions ("run the gate before committing", "pick up the next
issue in the milestone") for each agent that needs them. Nothing records a
prompt for reuse.

The bench has the same gap from the other side. Save to Bench stores a
template, and `create-agent` can deploy one over MCP, but the Rust app has no
way for the user to see, deploy or remove a bench entry, and no way to put an
agent away: Save to Bench copies it and leaves it running. The Swift app's
bench dropdown on the New Agent button was never ported.

## What Changes

- **Prompt library.** A named, reusable prompt collection (name + text),
  stored as its own durable-data document and managed from a new Prompts tab
  in Settings. Library prompts appear in the panel's `/` lookup; inserting one
  expands to its text in the composer.
- **Startup prompt.** An agent and a bench entry can each carry an optional
  startup prompt: either a reference to a library prompt or custom text. It is
  sent on every fresh session (never on resume), as its own turn after the
  initialization prompt's turn, which is unchanged. The agent editor gets a
  Startup Prompt control beside the persona picker. In the ACP panel it is
  queued like any other prompt and labelled as a startup prompt; for a
  terminal-view coding agent it is typed in at the idle after registration.
- **Bench an agent.** A new Bench Agent row-menu item saves the agent to the
  bench and removes it from the workspace, after a confirmation (its
  conversation is not kept). Save to Bench stays, unchanged, for copying.
- **Prompt variables.** Prompt text (library and custom) can use a closed
  set of variables - `{{agent.name}}`, `{{agent.id}}`, `{{agent.type}}`,
  `{{folder}}`, `{{folder.name}}`, `{{workspace}}`, `{{branch}}`,
  `{{date}}` - expanded against the agent a prompt is sent to, both when a
  startup prompt is sent and when a library prompt is inserted from `/`.
  Unknown names are left as typed and flagged in the editors; `\{{` writes
  a literal `{{`.
- **Bench UI port.** The Swift app's bench dropdown is ported to the
  sidebar's New Agent button, which becomes a split button: the main part
  opens the agent editor as today, and a chevron opens a popover with
  Create New Agent, the bench entries (avatar, name, folder; click to
  deploy; remove on hover with confirmation) and an empty-bench hint.
- **Deploy from the bench by menu.** The sidebar background menu gains a New
  from Bench submenu listing the same entries, for the same deployment.
- **Bench management.** A new Bench tab in Settings lists bench entries with
  rename, startup-prompt edit and remove.
- Bench templates, bench deployment and the durable agent field set gain the
  startup prompt, and it round-trips through Save to Bench and deployment
  (including `create-agent` with `benchAgentId`).

### Non-goals

- User-defined variables, variables that prompt for a value, and any logic
  in prompt text (conditionals, loops, filters).
- Adding a startup prompt parameter to the `create-agent` MCP tool; a
  deployed bench entry carries its own.
- Keeping a benched agent's conversation for resumption on redeploy.
- Changing the initialization (registration) prompt's text or delivery.
- Sharing, importing or exporting the prompt library, and importing Swift
  bench data beyond what `data-import` already does.
- Startup prompts for `shell`-type agents, which have no coding agent to
  prompt.

## Capabilities

### New Capabilities

- `prompt-library`: the prompt model (id, name, text), its durable document,
  add/update/remove semantics, how a startup prompt references a library
  prompt or holds custom text, how a reference to a deleted prompt resolves,
  and the prompt variables: their names, values, syntax, escaping and
  unknown-name handling.

### Modified Capabilities

- `settings-persistence`: the durable agent field set and bench templates
  gain an optional startup prompt; the prompt library is a seventh
  durable-data document.
- `settings-ui`: new Prompts tab (with a variable reference and
  unknown-variable warnings) and Bench tab.
- `agent-editor-ui`: new Startup Prompt control, with unknown-variable
  warnings on custom text.
- `agent-launch-command`: the startup prompt follows the initialization
  prompt on a fresh session, for both the ACP and the terminal paths.
- `agent-lifecycle`: bench deployment restores the startup prompt; new
  benching (save and remove) requirement.
- `agent-list-ui`: Bench Agent row-menu item and its visibility; the New
  Agent split button and its bench popover (the Swift dropdown port); New
  from Bench submenu in the sidebar background menu and its enablement.
- `panel-slash-commands`: library prompts are lookup entries, and inserting
  one expands to its text with variables resolved for the selected agent.
- `queued-message-management`: a queued startup prompt is labelled as one.

## Impact

- `knot-core`: `Prompt` record and `StartupPrompt` enum in
  `settings/records.rs`; `startup_prompt` on `SavedAgent` and `BenchAgent`
  (serde default `None`, so existing documents load unchanged); a `prompts`
  collection, its document path (`PROMPTS_FILE` in `consts.rs`) and
  `persist_prompts`; new `l10n` keys.
- `knot-agents`: `CreateOptions` and `deploy_bench` carry the startup
  prompt; a `bench_agent` store operation that removes the agent after the
  bench entry is written.
- `knot-agent-launch`: resolve a `StartupPrompt` against the library to text,
  expand variables against a `PromptContext`, and a one-line form for the
  terminal path. Variable names are a closed enum with `Display`/`FromStr`.
- `knot-git`: the current branch read (already present) is used for
  `{{branch}}`, off the UI thread.
- `knot-activity`: a startup prompt injected at the idle after registration.
- `knot`: panel session queues the startup prompt after the registration
  turn; agent editor control; row and background menu entries; the New
  Agent split button and bench popover; Settings Prompts and Bench tabs;
  slash-lookup source for library prompts with off-thread expansion.
- `knot-mcp-tools`: none beyond `deploy_bench` carrying the new field.
- No new dependencies.
