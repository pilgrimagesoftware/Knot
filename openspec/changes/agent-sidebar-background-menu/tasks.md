# Tasks

## 1. The item set, pure

- [ ] 1.1 Add `AgentListBackgroundEntry` and
      `sidebar_background_menu_entries(SidebarMenuFacts)` to
      `crates/knot/src/app_state.rs`, returning every entry with its enabled
      flag and the divider before Broadcast. Verify with unit tests covering
      an empty workspace (only New Agent enabled), a workspace whose agents
      are all stopped (Deactivate All disabled, the rest enabled), and a
      workspace with a running agent (all enabled).

## 2. Attaching the menu

- [ ] 2.1 Attach `.context_menu` to the sidebar's agent-list container in
      `crates/knot/src/workspace_window/mod.rs` and verify in the app that
      right-clicking below the rows opens it while right-clicking a row still
      opens that row's menu. If the outer menu fires over rows, move it to an
      explicit filler element sized to the list's remaining space, per
      design.md, and verify again.
- [ ] 2.2 Build the menu from `sidebar_background_menu_entries`, rendering
      disabled entries as disabled rather than omitting them. Verify in the
      app against an empty workspace that all five items are present and only
      New Agent is usable.

## 3. New Agent

- [ ] 3.1 Route New Agent to the same editor request the sidebar's existing
      "New agent" button raises, for the workspace the sidebar is showing.
      Verify in the app that it opens the same dialog with the same defaults,
      and that dismissing it creates nothing.

## 4. The bulk actions

- [ ] 4.1 Add Restart All: snapshot the workspace's agent ids, confirm with
      an alert naming the count, then on confirmation call the same restart
      path the row menu uses for each id and `persist_agents` once at the
      end. Verify in the app with three agents that all three restart, and
      that cancelling leaves all three untouched.
- [ ] 4.2 Add Close All the same way, reusing `remove_agent` per id. Verify
      in the app that the workspace ends empty, that companions of the closed
      agents go too, and that cancelling changes nothing.
- [ ] 4.3 Add Deactivate All with no confirmation, reusing `deactivate_agent`
      per running id. Verify in the app that every running agent stops, that
      a passive agent that never started is left alone, and that every agent
      is still listed and renders as not running.
- [ ] 4.4 Add a test that each bulk action iterates a snapshot rather than
      the live list, so removing agents mid-loop cannot leave half the
      workspace behind.

## 5. Broadcast

- [ ] 5.1 Extract `deliver_panel_prompt(&mut self, id, text)` out of
      `send_panel_prompt` - the send-or-queue half, including
      `record_user_message` - and have `send_panel_prompt` call it. Verify
      the existing panel prompt-queueing tests still pass unchanged; this
      task must be pure extraction with no behavior change.
- [ ] 5.2 Add the broadcast sheet: a multi-line field starting empty each
      time, Cancel, and a Send disabled while the trimmed message is empty.
      Verify in the app that Send is greyed on an empty or whitespace-only
      message, that Escape discards, and that reopening the sheet after a
      send shows an empty field.
- [ ] 5.3 Bind Send to modifier-Return and leave plain Return inserting a
      newline, following whichever key mechanism the
      `workspace-dialog-keyboard` change landed on. Verify both in the app.
- [ ] 5.4 On send, deliver the trimmed text to every agent in the workspace:
      `deliver_panel_prompt` for a panel agent, `TerminalSession::send_text`
      for a terminal agent, skip an agent with neither. Verify in the app
      with one idle and one mid-turn panel agent that the idle one answers at
      once, the mid-turn one is not interrupted, and the message arrives when
      its turn ends.
- [ ] 5.5 Verify in the app that a broadcast appears in each agent's
      conversation as a user message, indistinguishable from the same text
      typed into that agent's own composer.

## 6. Verification

- [ ] 6.1 Run `make rust` and verify fmt, clippy, tests and build all pass
      for the workspace.
- [ ] 6.2 Walk the whole menu once in a workspace of three agents - New
      Agent, Restart All, Deactivate All, Broadcast, Close All - and confirm
      each does what the spec says and that the enablement rules hold as the
      workspace empties.
