## 1. The item set, shared

- [ ] 1.1 Add one unit action per `AgentMenuEntry` that has a label, and a
      total mapping between the two. Verify with a test that every entry
      with a label has an action and every action maps back to exactly one
      entry - so an item added to the context menu cannot silently skip the
      menu bar.

## 2. The menu

- [ ] 2.1 Build the Agents menu from `agent_context_menu_entries` with the
      facts of the selected agent, showing every item and marking those that
      do not apply as disabled, and insert it into the menu bar after View.
      Verify with a test that the menu's item order and grouping match the
      entry list, and that an entry hidden by the context menu appears here
      disabled rather than absent.
- [ ] 2.2 Build the Move to Workspace and Markdown Files submenus from the
      same sources the context menu uses. Verify in the app that both list
      what the context menu lists for the same agent.

## 3. Enablement

- [ ] 3.1 Register the handlers on the workspace window while an agent is
      selected, on an element in the focus path, and **not** globally.
      Verify in the app: items are disabled with nothing selected, enable on
      selecting an agent, and are disabled again when only the workspace
      manager is focused.
- [ ] 3.2 Route each action to `run_agent_menu_action` for the selected
      agent, so both menus run the same code. Verify in the app that Remove
      Agent from the menu bar asks the same confirmation and removes the same
      agent as from the row.

## 4. Keeping the submenus current

- [ ] 4.1 Rebuild the menu bar when the facts behind the dynamic submenus
      change - the workspace list, or the selected agent's markdown history -
      and not on every poll. Verify in the app: create a second workspace and
      confirm Move to Workspace offers it without relaunching.

## 5. Verification

- [ ] 5.1 `make rust` passes clean.
- [ ] 5.2 Confirm a dialog opened from the menu bar appears: the handler runs
      inside the active window's update, so an undeferred `open_alert_dialog`
      fails with "window not found" and the item looks dead - the About Knot
      bug. Exercise Remove and Restart from the menu bar specifically.
