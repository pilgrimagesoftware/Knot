use super::*;
pub(crate) struct AgentEditorRequest {
    pub(crate) workspace_id:   Uuid,
    /// Prefilled folder for a new agent, the way the Swift reference's
    /// `addAgent(to:)` does when launched from a dashboard's "Add Agent"
    /// tile: folder copied from an existing agent in the workspace. Ignored
    /// when `edit_target` is set.
    pub(crate) prefill_folder: Option<String>,
    /// Where to insert a newly created agent. Ignored when `edit_target` is
    /// set - editing never moves an agent's position.
    pub(crate) insert_after:   Option<Uuid>,
    /// `Some(id)` opens the dialog in edit mode for that existing agent
    /// (prefilled from it, submitting via `AgentStore::edit`) instead of
    /// creating a new one.
    pub(crate) edit_target:    Option<Uuid>,
}

/// Opens the agent editor dialog, in create or edit mode depending on
/// `request.edit_target`. `on_created` is called with the new (create mode)
/// or edited (edit mode) agent's id once the dialog is submitted.
pub(crate) fn open_agent_editor(store: Arc<Mutex<knot_agents::AgentStore>>,
                                settings: knot_core::Settings, request: AgentEditorRequest,
                                on_created: impl Fn(Uuid, &mut Window, &mut App) + 'static,
                                cx: &mut App) {
    let AgentEditorRequest { workspace_id,
                             prefill_folder,
                             insert_after,
                             edit_target, } = request;
    // Re-read from disk rather than trusting the caller's copy. Every
    // window holds its own `Settings` snapshot taken when it opened, and
    // personas are edited in a different window that persists to disk -
    // so a persona added or renamed since this window opened was missing
    // from the picker, which is the whole content of this dialog's
    // persona field. Falls back to the caller's snapshot if the file
    // can't be read.
    let settings = knot_core::Settings::load().unwrap_or(settings);
    let editing = edit_target.and_then(|id| store.lock().unwrap().agent(id).cloned());
    let title = if editing.is_some() {
        "Edit Agent"
    }
    else {
        "New Agent"
    };
    let options = agent_window_options(title, cx);
    let _ =
        cx.open_window(options, move |window, cx| {
              let name_input = cx.new(|cx| {
                                     InputState::new(window, cx)
                .placeholder("Name")
                .default_value(editing.as_ref().map(|a| a.name.clone()).unwrap_or_default())
                                 });
              let shell_command_input =
                  cx.new(|cx| InputState::new(window, cx).placeholder("Shell command (optional)"));
              let avatar_input = cx.new(|cx| {
                                       InputState::new(window, cx).default_value(
                editing
                    .as_ref()
                    .map(|a| a.avatar.clone())
                    .unwrap_or_else(|| "🤖".to_string()),
            )
                                   });
              let view = cx.new(|cx| {
                               let avatar_subscription = cx.subscribe_in(
                &avatar_input,
                window,
                |this: &mut AgentEditor, avatar_input, event, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.clamp_avatar_to_one_character(avatar_input, window, cx);
                    }
                },
            );
                               let name_subscription =
                                   cx.subscribe(&name_input,
                                                |_: &mut AgentEditor, _, event, cx| {
                                                    if matches!(event, InputEvent::Change) {
                                                        cx.notify();
                                                    }
                                                });
                               let persona_id = editing.as_ref().and_then(|a| a.persona_id);
                               AgentEditor { store,
                                             settings,
                                             workspace_id,
                                             name_input,
                                             shell_command_input,
                                             avatar_input,
                                             _avatar_subscription: avatar_subscription,
                                             _name_subscription: name_subscription,
                                             folder_path: editing.as_ref()
                                                                 .map(|a| a.folder.clone())
                                                                 .or(prefill_folder)
                                                                 .unwrap_or_default(),
                                             agent_type:
                                                 editing.as_ref()
                                                        .map(|a| a.agent_type.clone())
                                                        .unwrap_or_else(|| "claude".to_string()),
                                             persona_id,
                                             original_persona_id: persona_id,
                                             insert_after,
                                             edit_target,
                                             on_created: Box::new(on_created),
                                             error: None }
                           });
              cx.new(|cx| Root::new(view, window, cx).bg(cx.theme().background))
          });
}

pub(crate) struct AgentEditor {
    store:                Arc<Mutex<knot_agents::AgentStore>>,
    settings:             knot_core::Settings,
    workspace_id:         Uuid,
    name_input:           Entity<InputState>,
    shell_command_input:  Entity<InputState>,
    avatar_input:         Entity<InputState>,
    _avatar_subscription: Subscription,
    _name_subscription:   Subscription,
    folder_path:          String,
    agent_type:           String,
    persona_id:           Option<Uuid>,
    /// Snapshot of `persona_id` when the dialog opened, so `save_edit` can
    /// tell `AgentStore::edit` whether the persona actually changed
    /// (`EditRequest::persona_changed`) rather than always forcing a
    /// restart.
    original_persona_id:  Option<Uuid>,
    insert_after:         Option<Uuid>,
    /// `Some(id)` when editing an existing agent instead of creating one -
    /// gates prefill, the submit button's label/handler, and whether the
    /// (edit-unsupported) shell-command field shows at all.
    edit_target:          Option<Uuid>,
    on_created:           Box<AgentCreatedCallback>,
    error:                Option<String>,
}

pub(crate) type AgentCreatedCallback = dyn Fn(Uuid, &mut Window, &mut App);

impl AgentEditor {
    /// Whether the form has everything required to submit - the primary
    /// button ("Add Agent" or "Save") is disabled until this is true.
    fn can_submit(&self, cx: &Context<Self>) -> bool {
        !self.name_input.read(cx).value().trim().is_empty()
        && !self.folder_path.trim().is_empty()
        && PathBuf::from(self.folder_path.trim()).is_dir()
    }

    fn create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let folder = self.folder_path.trim().to_string();
        if folder.is_empty() || !PathBuf::from(&folder).is_dir() {
            self.error = Some("Choose a folder.".to_string());
            cx.notify();
            return;
        }
        let name = self.name_input.read(cx).value().trim().to_string();
        if name.is_empty() {
            self.error = Some("Enter a name.".to_string());
            cx.notify();
            return;
        }
        let avatar = self.avatar_input.read(cx).value().trim().to_string();
        let agent_type = self.agent_type.clone();
        let shell_command = self.shell_command_input.read(cx).value().trim().to_string();
        let created_id = {
            let mut store = self.store.lock().unwrap();
            store.set_current_workspace(self.workspace_id);
            let id = store.create(
                folder,
                knot_agents::CreateOptions {
                    name: Some(name),
                    avatar: (!avatar.is_empty()).then_some(avatar),
                    agent_type: (!agent_type.is_empty()).then_some(agent_type),
                    shell_command: (!shell_command.is_empty()).then_some(shell_command),
                    persona_id: self.persona_id,
                    insert_after: self.insert_after,
                    ..Default::default()
                },
            );
            self.settings.saved_agents =
                store.saved_agents(self.settings.restore_conversation_on_launch);
            self.settings.saved_workspaces = store.saved_workspaces();
            id
        };
        let _ = self.settings.persist();
        (self.on_created)(created_id, window, cx);
        window.remove_window();
    }

    /// Applies edits to `self.edit_target` via `AgentStore::edit`. The
    /// shell-command field isn't submitted - `EditRequest` has no field for
    /// it, so the row is hidden in edit mode (see `Render for AgentEditor`).
    fn save_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.edit_target
        else {
            return;
        };
        let folder = self.folder_path.trim().to_string();
        if folder.is_empty() || !PathBuf::from(&folder).is_dir() {
            self.error = Some("Choose a folder.".to_string());
            cx.notify();
            return;
        }
        let name = self.name_input.read(cx).value().trim().to_string();
        if name.is_empty() {
            self.error = Some("Enter a name.".to_string());
            cx.notify();
            return;
        }
        let avatar = self.avatar_input.read(cx).value().trim().to_string();
        let agent_type = self.agent_type.clone();
        let persona_changed = self.persona_id != self.original_persona_id;
        {
            let mut store = self.store.lock().unwrap();
            let result = store.edit(
                id,
                knot_agents::EditRequest {
                    name,
                    avatar,
                    folder: Some(folder),
                    agent_type: (!agent_type.is_empty()).then_some(agent_type),
                    persona_id: self.persona_id,
                    persona_changed,
                    relocate_companions: false,
                },
            );
            if let Err(error) = result {
                self.error = Some(error.to_string());
                cx.notify();
                return;
            }
            self.settings.saved_agents =
                store.saved_agents(self.settings.restore_conversation_on_launch);
            self.settings.saved_workspaces = store.saved_workspaces();
        }
        let _ = self.settings.persist();
        (self.on_created)(id, window, cx);
        window.remove_window();
    }

    fn choose_folder(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose Agent Folder".into()),
        });
        let editor = cx.entity();
        cx.spawn(async move |_this, cx| {
              let Ok(Ok(Some(paths))) = receiver.await
              else {
                  return;
              };
              let Some(path) = paths.into_iter().next()
              else {
                  return;
              };
              cx.update(|app| {
                    editor.update(app, |editor, cx| {
                              editor.folder_path = path.to_string_lossy().into_owned();
                              cx.notify();
                          });
                });
          })
          .detach();
    }

    /// Clears the avatar field, then focuses it and opens the OS character
    /// picker - it inserts the chosen character into whatever field has
    /// keyboard focus, so clearing first makes the picker replace the
    /// current avatar rather than append to it.
    fn choose_avatar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.avatar_input.update(cx, |input, cx| {
                             input.set_value("", window, cx);
                             input.focus(window, cx);
                         });
        #[cfg(target_os = "macos")]
        native_character_picker::open();
    }

    /// Keeps the avatar field to a single character (grapheme cluster), so
    /// typing or pasting past one character doesn't silently grow it.
    fn clamp_avatar_to_one_character(&mut self, avatar_input: &Entity<InputState>,
                                     window: &mut Window, cx: &mut Context<Self>) {
        let value = avatar_input.read(cx).value().to_string();
        let Some(first) = value.graphemes(true).next()
        else {
            return;
        };
        if first.len() == value.len() {
            return;
        }
        let first = first.to_string();
        avatar_input.update(cx, |input, cx| {
                        input.set_value(first, window, cx);
                    });
    }
}

impl AgentEditor {
    /// A `LabeledContent`-style row: label at the leading edge, control(s)
    /// trailing - matching the Swift reference's `Form` rows, as opposed to
    /// the Settings window's fixed right-aligned label column.
    fn dialog_row(label: &'static str, control: impl IntoElement) -> impl IntoElement {
        h_flex().justify_between()
                .items_center()
                .gap_3()
                .child(div().child(label))
                .child(control)
    }

    /// A card grouping related rows - the Swift reference's `Form` sections
    /// use a filled, borderless card rather than the Settings window's
    /// titled, outlined `GroupBox`.
    fn dialog_section(_cx: &Context<Self>, rows: Vec<gpui_kit::AnyElement>) -> impl IntoElement {
        GroupBox::new().fill()
                       .child(v_flex().gap_3().children(rows).into_any_element())
    }
}

impl Render for AgentEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let editor = cx.entity();
        let personas = self.settings.personas.clone();
        let is_shell = self.agent_type == "shell";

        let identity_rows = vec![
            Self::dialog_row("Name", Input::new(&self.name_input).w(px(200.))).into_any_element(),
            Self::dialog_row(
                "Avatar",
                h_flex()
                    .gap_2()
                    .child(Input::new(&self.avatar_input).w(px(48.)))
                    .child(
                        SettingsWindow::icon_button(
                            "agent-avatar-picker",
                            "icons/face-grinning.svg",
                            "Choose character…",
                            false,
                        )
                        .on_click(
                            cx.listener(|editor, _, window, cx| editor.choose_avatar(window, cx)),
                        ),
                    ),
            )
            .into_any_element(),
        ];

        let mut agent_rows = vec![
            Self::dialog_row(
                "Coding agent",
                Button::new("agent-type-picker")
                    .label(SettingsWindow::agent_type_label(&self.agent_type))
                    .dropdown_caret(true)
                    .dropdown_menu({
                        let editor = editor.clone();
                        move |mut menu, _, _| {
                            // Matches the Swift reference's `availableAgents`
                            // list (`CodingSettingsView.swift`).
                            for (agent_type, label) in [
                                ("claude", "Claude"),
                                ("codex", "Codex"),
                                ("opencode", "OpenCode"),
                                ("gemini", "Gemini"),
                                ("copilot", "Copilot"),
                                ("custom1", "Custom 1"),
                                ("custom2", "Custom 2"),
                                ("shell", "Shell"),
                            ] {
                                let editor = editor.clone();
                                menu = menu.item(PopupMenuItem::new(label).on_click(
                                    move |_, _, app| {
                                        editor.update(app, |e, _| {
                                            e.agent_type = agent_type.to_string()
                                        })
                                    },
                                ));
                            }
                            menu
                        }
                    }),
            )
            .into_any_element(),
        ];
        if is_shell && self.edit_target.is_none() {
            agent_rows.push(
                Self::dialog_row(
                    "Command",
                    Input::new(&self.shell_command_input)
                        .w(px(200.))
                        .font_family(cx.theme().mono_font_family.clone()),
                )
                .into_any_element(),
            );
        }
        if !personas.is_empty() {
            agent_rows.push(
                Self::dialog_row(
                    "Persona",
                    Button::new("agent-persona-picker")
                        .label(
                            self.persona_id
                                .and_then(|id| {
                                    personas.iter().find(|p| p.id == id).map(|p| p.name.clone())
                                })
                                .unwrap_or_else(|| "None".to_string()),
                        )
                        .dropdown_caret(true)
                        .dropdown_menu({
                            let editor = editor.clone();
                            move |mut menu, _, _| {
                                menu = menu.item(PopupMenuItem::new("None").on_click({
                                    let editor = editor.clone();
                                    move |_, _, app| editor.update(app, |e, _| e.persona_id = None)
                                }));
                                for persona in &personas {
                                    let id = persona.id;
                                    menu = menu.item(
                                        PopupMenuItem::new(persona.name.clone()).on_click({
                                            let editor = editor.clone();
                                            move |_, _, app| {
                                                editor.update(app, |e, _| e.persona_id = Some(id))
                                            }
                                        }),
                                    );
                                }
                                menu
                            }
                        }),
                )
                .into_any_element(),
            );
        }

        let folder_rows = vec![
            Self::dialog_row(
                "Folder",
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .max_w(px(220.))
                            .text_sm()
                            .whitespace_normal()
                            .text_color(cx.theme().muted_foreground)
                            .child(if self.folder_path.is_empty() {
                                "No folder selected".to_string()
                            } else {
                                self.folder_path.clone()
                            }),
                    )
                    .child(
                        SettingsWindow::icon_button(
                            "choose-agent-folder",
                            "icons/folder-open.svg",
                            "Choose folder…",
                            false,
                        )
                        .on_click(cx.listener(|editor, _, _, cx| editor.choose_folder(cx))),
                    ),
            )
            .into_any_element(),
        ];

        v_flex()
            .size_full()
            .gap_3()
            .px_5()
            .pt_5()
            .pb_6()
            .bg(cx.theme().background)
            .child(
                // Scrolls in place instead of pushing the action row (which
                // must stay visible) off the bottom of the window - the
                // folder path row can wrap to more than one line.
                div()
                    .id("new-agent-content")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(
                        v_flex()
                            .gap_3()
                            .child(Self::dialog_section(cx, identity_rows))
                            .child(Self::dialog_section(cx, agent_rows))
                            .child(Self::dialog_section(cx, folder_rows))
                            .children(self.error.as_ref().map(|error| {
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().danger)
                                    .child(error.clone())
                            })),
                    ),
            )
            .child(
                h_flex()
                    .flex_shrink_0()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("cancel-agent-editor")
                            .label("Cancel")
                            .on_click(|_, window, _| window.remove_window()),
                    )
                    .child(
                        Button::new("create-agent-editor")
                            .label(if self.edit_target.is_some() {
                                "Save"
                            } else {
                                "Add Agent"
                            })
                            .primary()
                            .disabled(!self.can_submit(cx))
                            .on_click(cx.listener(|editor, _, window, cx| {
                                if editor.edit_target.is_some() {
                                    editor.save_edit(window, cx);
                                } else {
                                    editor.create(window, cx);
                                }
                            })),
                    ),
            )
            .children(crate::app_support::root_overlays(window, cx))
    }
}
