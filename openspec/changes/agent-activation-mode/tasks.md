## 1. The model

- [x] 1.1 Add an `ActivationMode` enum (`Active`, `Passive`) to `knot-core`,
      serialized in the settings file's camelCase style, and a
      `activation_mode` field on `SavedAgent` whose serde default is
      `Active` - with the reason in a comment beside it. Verify with a test
      that a saved-agent JSON blob with no `activationMode` key
      deserializes as `Active`, and that one written by this build round
      trips.
- [x] 1.2 Add `activation_mode` to `knot_agents::Agent` (durable, carried
      through `from_saved`/`saved_agents`) and a runtime-only
      `activated: bool` that `from_saved` always sets false. Verify with a
      test that persisting and reloading a running passive agent yields
      `activation_mode == Passive` and `activated == false`.
- [x] 1.3 Add `activation_mode` to `CreateOptions`, defaulting to `Passive`,
      and honour it in `AgentStore::create`. Verify with a test that
      `CreateOptions::default()` creates a passive agent - the deliberate
      disagreement with 1.1's load default.
- [x] 1.4 Add `activation_mode` to `EditRequest` and apply it in
      `AgentStore::edit` **without** setting `restart_required`. Verify with
      a test that changing only the mode leaves the agent's restart token
      unchanged.

## 2. Activation and deactivation

- [x] 2.1 Add `AgentStore::set_activated(id, bool)` and have `create` mark an
      `Active` agent activated at creation, so it starts when its workspace
      next opens. Verify with a test covering both modes.
- [x] 2.2 Extract the session teardown that `WorkspaceWindow::remove_agent`
      performs - `remove_session`, panel state, prompt input and its
      subscription, scroll handle, pending context, input-expanded flag -
      into one function, and call it from `remove_agent` so behaviour is
      unchanged. Verify `make rust` still passes and the existing removal
      tests are untouched.
- [x] 2.3 Add `WorkspaceWindow::deactivate_agent(id)`: deactivate owned
      companions first, then run 2.2's teardown for the agent and clear its
      `activated` flag, leaving the store's agent and workspace membership
      alone. Verify with a store-level test that the agent and its
      companions survive with their ordering and workspace intact.
- [x] 2.4 Gate `ensure_session` and `ensure_panel_session` on the agent being
      activated, so every existing caller inherits the rule. Verify by
      reading the call sites that none of them start an unactivated agent,
      and that the existing shell-agent path still starts normally.
- [x] 2.5 Mark an agent activated when its sidebar row is selected. Verify in
      the running app that a passive agent starts on first selection and a
      deactivated one restarts on the next.

## 3. What the user sees

- [x] 3.1 Add the stopped placeholder to the content pane for a selected
      agent that is not activated, naming why it is not running and that
      selecting it starts it. Verify in the app on a fresh passive agent and
      after Deactivate.
- [x] 3.2 Give a sidebar row for an agent that is not running a distinct
      treatment from a running one, keyed on liveness rather than activation
      mode, and not on the state dot. Verify in the app with a workspace
      holding a running agent, an unstarted passive agent and a deactivated
      one - the first distinguishable from the other two, which match.
- [x] 3.3 Add `Deactivate` to the agent context menu: shown only while the
      agent is running, immediately above Restart Agent, no confirmation.
      Extend `agent_context_menu_entries` and its fact struct, and verify
      with tests that it is absent for a stopped agent, present for a
      running one, and that the exhaustive divider test still holds.
- [x] 3.4 Add the activation segmented control and its hint to the agent
      editor, defaulting to `Passive` when creating and to the agent's own
      mode when editing, wired through `CreateOptions`/`EditRequest`. Verify
      in the app that a new agent is created passive and that editing an
      existing agent shows its real mode.

## 4. Verification

- [ ] 4.1 Confirm layout restore does not activate a passive agent: restoring
      a layout sets the selection without going through the activation path.
      Verify by relaunching with a restored workspace holding a passive
      agent and confirming no adapter subprocess is spawned for it
      (`ps` for the adapter, or the absence of its connecting pane).
- [ ] 4.2 Confirm an existing settings file is unaffected: with a settings
      file written before this change, every agent still starts when its
      workspace opens.
- [x] 4.3 `make rust` passes clean.
- [ ] 4.4 Manually, in the app: open a workspace of mixed agents and confirm
      only the active ones start; activate a passive one; deactivate it;
      deactivate an owner with companions; change a running agent's mode and
      confirm it keeps running.
