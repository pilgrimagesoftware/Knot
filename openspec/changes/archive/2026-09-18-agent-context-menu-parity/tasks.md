## 1. Item set

- [x] 1.1 Model the menu as typed entries rather than label strings, with
      dividers emitted only between non-empty groups.
- [x] 1.2 Cover the order, each visibility rule, and divider placement
      across every combination of the facts.

## 2. Backing the new items need

- [x] 2.1 `AgentPrefill`: let the editor open on an agent derived from an
      existing one (owner, companion flag, session), which Fork Agent and
      New Companion both need.
- [x] 2.2 `AgentStore::fork_session`: point a not-yet-started agent at an
      existing session with the fork flag, leaving the source's own session
      alone.
- [x] 2.3 `open_in`: the Open In… targets and their `open` arguments.
- [x] 2.4 A markdown pane, so Markdown Files has somewhere to show a file -
      `markdown_file` had no reader at all, which also left the
      `display-markdown` MCP tool writing state nothing displayed.
- [x] 2.5 `AgentStore::clear_markdown_panel`, to close that pane without
      losing the history the menu lists.

## 3. The menu

- [x] 3.1 New Companion… - editor, pre-filled as a companion of the row.
- [x] 3.2 Fork Agent - editor, pre-filled from the row and carrying its
      session.
- [x] 3.3 Duplicate Agent - immediate, no session carried.
- [x] 3.4 Move to Workspace - submenu of attached workspaces except the
      agent's own.
- [x] 3.5 Save to Bench - against freshly loaded settings, so the bench
      does not lose entries added in the settings window.
- [x] 3.6 Open In… - submenu.
- [x] 3.7 Markdown Files - submenu of the agent's history, newest first.
- [x] 3.8 Register Agent - sends the registration prompt over the agent's
      panel session.
- [x] 3.9 Keep Restart and Remove behind their existing confirmations.
- [x] 3.10 Test `agent_menu_facts`, the store-reading half: move targets,
      detached workspaces, markdown history, and a missing agent.

## 4. Verification

- [x] 4.1 `make rust` passes clean.
- [ ] 4.2 Manually, in the running app: open the menu on a plain agent, a
      companion, and a shell agent, and confirm each shows the item set
      this change specifies with no stray dividers.
- [ ] 4.3 Manually: exercise each new item once - the two editor-opening
      ones, Duplicate, a move between two workspaces, Save to Bench, one
      Open In target, a markdown file, and Register on a connected agent.

Both manual tasks need someone watching the app; see the note in
`openspec/changes/acp-only-agent-launch/tasks.md` for why an agent session
cannot close them here.
