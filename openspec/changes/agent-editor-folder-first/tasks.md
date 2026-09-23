# Tasks

## 1. One definition of a folder's name

- [x] 1.1 Move `last_path_component` from `crates/knot-agents/src/store/helpers.rs`
      to `knot-core`, beside the other rules both crates read, and have
      `AgentStore::create` call it there; verify with
      `cargo test -p knot-agents create_from_folder_with_defaults`, which
      already pins that a folderful create is named after its folder
- [x] 1.2 Move the helper's tests with it rather than leaving them behind, and
      add one for the `None` case - a path with no last component - since that
      is the case the editor now depends on; verify with
      `cargo test -p knot-core`

## 2. The name follows the folder

- [ ] 2.1 Give `choose_folder` a `&mut Window`, switch its `cx.spawn` to
      `cx.spawn_in(window, ..)` and its `cx.update` to `this.update_in(..)`,
      and delete the underscore at the call site in `render.rs` that is
      already discarding a window; verify with `cargo build -p knot`, which
      fails until the one call site is updated
- [ ] 2.2 Fill the name field inside that `update_in` when the trimmed name is
      empty and the chosen folder has a last component, leaving a non-blank
      name untouched. Read the field inside the closure, not before the spawn:
      a name typed while the picker was open must win; verify with the tests
      in 2.3
- [ ] 2.3 Add tests for the four cases the spec names - blank name takes the
      folder's name, a typed name survives, a second folder does not rename an
      already-filled field, and a whitespace-only name counts as blank - plus
      the no-last-component case leaving the field alone. Drive the input
      through `update_in` the way `tests/workspace_dialog.rs` already does;
      verify with `cargo test -p knot`

## 3. The folder comes second

- [ ] 3.1 Move the `folder_rows` section between the identity and agent
      sections in `Render for AgentEditor`, leaving the other three in their
      current order; verify by reading the four `dialog_section` children back
      in order
- [ ] 3.2 Update the scroll container's comment, which explains itself in
      terms of the folder path row being the last thing in the form and is
      wrong once it is not; verify by reading it back against the new order

## 4. Walk the spec

- [ ] 4.1 Open the dialog to create an agent and confirm the folder is
      presented after the name and avatar and before the agent type, persona,
      activation mode and registry metadata
- [ ] 4.2 With the name blank, choose a folder and confirm the name field
      shows that folder's last component, that it can be edited before
      submitting, and that submitting creates an agent with the edited name
- [ ] 4.3 Type a name, then choose a folder, and confirm the typed name is
      still there; then clear the name and submit, and confirm the dialog
      still refuses an unnamed agent with the message it always used
- [ ] 4.4 Confirm the dialog is still refused with no folder and with a folder
      that is not an existing directory, with the same messages as before -
      this change moves when the name is filled, not what is required
- [ ] 4.5 Run `make` and confirm the whole gate passes - `fmt-check`,
      `size-check`, `clippy -D warnings`, tests, build
