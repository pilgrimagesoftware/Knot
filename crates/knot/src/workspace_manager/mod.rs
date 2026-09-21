use super::*;

/// Whether a typed workspace name counts as nothing at all.
///
/// The confirm button's disabled state and the Return key both read this,
/// so a name the button refuses is a name Return refuses, by construction
/// rather than by two guards kept in step by hand. Judged on the trimmed
/// form, which is also the form `save_name` stores.
pub(crate) fn workspace_name_is_blank(name: &str) -> bool {
    name.trim().is_empty()
}

pub(crate) struct WorkspaceManager {
    pub(crate) store:                 Arc<Mutex<knot_agents::AgentStore>>,
    pub(crate) messages:              Arc<Mutex<knot_messaging::MessageStore>>,
    pub(crate) settings:              knot_core::Settings,
    pub(crate) name_input:            Entity<InputState>,
    pub(crate) editing_id:            Option<Uuid>,
    pub(crate) workspace_dialog_id:   Option<Uuid>,
    pub(crate) show_workspace_dialog: bool,
    pub(crate) delete_workspace_id:   Option<Uuid>,
    pub(crate) error:                 Option<String>,
    /// Keeps the confirm button's disabled state honest while the user
    /// types: without it the button only re-reads the name on the next
    /// unrelated re-render, so an empty field could stay greyed after the
    /// first character.
    pub(crate) _name_subscription:    Subscription,
    pub(crate) _mcp_stop:             Option<tokio::sync::oneshot::Sender<()>>,
}

#[derive(Clone)]
pub(crate) struct WorkspaceDrag(Uuid);

pub(crate) struct WorkspaceDragPreview;

impl Render for WorkspaceDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().p_2().child("Workspace")
    }
}

impl WorkspaceManager {
    fn persist(&mut self) {
        let Ok(store) = self.store.lock()
        else {
            self.error = Some("Agent store is unavailable.".to_string());
            return;
        };
        self.settings.saved_agents =
            store.saved_agents(self.settings.restore_conversation_on_launch);
        self.settings.saved_workspaces = store.saved_workspaces();
        if let Err(error) = self.settings.persist() {
            self.error = Some(format!("Could not save workspace: {error}"));
        }
    }

    fn save_name(&mut self, name: String, editing_id: Option<Uuid>, window: &mut Window,
                 cx: &mut Context<Self>) {
        if name.is_empty() {
            self.error = Some("Workspace name cannot be empty.".to_string());
            cx.notify();
            return;
        }
        let mut store = self.store.lock().unwrap();
        if let Some(id) = editing_id {
            if !store.rename_workspace(id, name) {
                self.error = Some("Workspace no longer exists.".to_string());
                cx.notify();
                return;
            }
        }
        else {
            let id = Uuid::new_v4();
            store.add_workspace(knot_core::Workspace { id,
                                                       name,
                                                       color_hex: "#1B4FB2".to_string(),
                                                       agent_ids: Vec::new(),
                                                       layout_mode: "single".to_string(),
                                                       active_agent_ids: Vec::new(),
                                                       focused_pane_index: 0,
                                                       split_ratio: 0.5,
                                                       split_ratio_secondary: None,
                                                       show_dashboard: None,
                                                       is_detached: None,
                                                       window_bounds: None });
            store.set_current_workspace(id);
        }
        drop(store);
        self.persist();
        self.editing_id = None;
        self.error = None;
        cx.update_entity(&self.name_input, |input, input_cx| {
              input.clean(window, input_cx);
          });
        cx.notify();
    }

    pub(crate) fn open_workspace_dialog(&mut self, editing_id: Option<Uuid>,
                                        window: &mut Window, cx: &mut Context<Self>) {
        let name =
            editing_id.and_then(|id| {
                          self.store.lock().ok().and_then(|store| {
                                                    store.workspaces()
                                                         .iter()
                                                         .find(|workspace| workspace.id == id)
                                                         .map(|workspace| workspace.name.clone())
                                                })
                      })
                      .unwrap_or_default();
        self.workspace_dialog_id = editing_id;
        self.show_workspace_dialog = true;
        self.error = None;
        cx.update_entity(&self.name_input, |input, input_cx| {
              input.set_value(name, window, input_cx);
              input.focus(window, input_cx);
          });
        cx.notify();
    }

    fn cancel_workspace_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.workspace_dialog_id = None;
        self.show_workspace_dialog = false;
        self.error = None;
        cx.update_entity(&self.name_input, |input, input_cx| {
              input.clean(window, input_cx);
          });
        cx.notify();
    }

    fn confirm_workspace_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.name_input.read(cx).value().trim().to_string();
        let editing_id = self.workspace_dialog_id;
        self.save_name(name, editing_id, window, cx);
        if self.error.is_none() {
            self.workspace_dialog_id = None;
            self.show_workspace_dialog = false;
        }
        cx.notify();
    }

    fn delete(&mut self, id: Uuid, cx: &mut Context<Self>) {
        if !self.store.lock().unwrap().remove_workspace(id) {
            self.error = Some("At least one workspace must remain.".to_string());
        }
        else {
            self.persist();
            self.error = None;
        }
        cx.notify();
    }

    fn request_delete(&mut self, id: Uuid, cx: &mut Context<Self>) {
        self.delete_workspace_id = Some(id);
        self.error = None;
        cx.notify();
    }

    fn cancel_delete(&mut self, cx: &mut Context<Self>) {
        self.delete_workspace_id = None;
        cx.notify();
    }

    fn confirm_delete(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.delete_workspace_id.take()
        else {
            return;
        };
        self.delete(id, cx);
    }

    fn move_before(&mut self, id: Uuid, target_id: Uuid, cx: &mut Context<Self>) {
        if self.store
               .lock()
               .unwrap()
               .move_workspace_before(id, target_id)
        {
            self.persist();
            cx.notify();
        }
    }

    fn select(&mut self, id: Uuid, cx: &mut Context<Self>) {
        self.store.lock().unwrap().set_current_workspace(id);
        cx.notify();
    }

    fn open(&mut self, id: Uuid, cx: &mut Context<Self>) {
        self.select(id, cx);
        WorkspaceWindow::open(Arc::clone(&self.store),
                              Arc::clone(&self.messages),
                              self.settings.clone(),
                              id,
                              cx);
    }
}

impl Render for WorkspaceManager {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let workspaces = self.store.lock().unwrap().workspaces().to_vec();
        let delete_name =
            self.delete_workspace_id.and_then(|id| {
                                        workspaces.iter()
                                                  .find(|workspace| workspace.id == id)
                                                  .map(|workspace| workspace.name.clone())
                                    });
        let name_is_blank = workspace_name_is_blank(&self.name_input.read(cx).value());
        let rows = workspaces.into_iter().map(|workspace| {
                                             let id = workspace.id;
                                             let agent_count = workspace.agent_ids.len();
                                             let selected =
                                                 self.store.lock().unwrap().current_workspace_id()
                                                 == Some(id);
                                             h_flex()
                .id(format!("workspace-row-{id}"))
                .on_drop(
                    cx.listener(move |manager, drag: &WorkspaceDrag, _window, cx| {
                        manager.move_before(drag.0, id, cx);
                    }),
                )
                // Double-click opens, matching the row's own "Open
                // workspace" button; a single click only selects, so a
                // click on the way to a rename or delete doesn't open a
                // window.
                .on_click(
                    cx.listener(move |manager, event: &ClickEvent, _window, cx| {
                        if event.click_count() >= 2 {
                            manager.open(id, cx);
                        } else {
                            manager.select(id, cx);
                        }
                    }),
                )
                .cursor_pointer()
                .w_full()
                .items_center()
                .gap_3()
                .p_3()
                .rounded(cx.theme().radius)
                .bg(if selected {
                    cx.theme().muted
                } else {
                    cx.theme().transparent
                })
                .child(
                    v_flex()
                        .flex_1()
                        .gap_1()
                        .child(div().text_lg().child(workspace.name.clone()))
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("{agent_count} agents")),
                        ),
                )
                .child(
                    Button::new(format!("open-workspace-{id}"))
                        .icon(IconName::ExternalLink)
                        .ghost()
                        .tooltip("Open workspace")
                        .on_click(cx.listener(move |manager, _: &ClickEvent, _window, cx| {
                            manager.open(id, cx);
                        })),
                )
                .child(
                    Button::new(format!("rename-workspace-{id}"))
                        .icon(IconName::FileText)
                        .ghost()
                        .tooltip("Rename workspace")
                        .on_click(cx.listener(move |manager, _: &ClickEvent, window, cx| {
                            manager.open_workspace_dialog(Some(id), window, cx);
                        })),
                )
                .child(
                    Button::new(format!("delete-workspace-{id}"))
                        .icon(IconName::Delete)
                        .danger()
                        .tooltip("Delete workspace")
                        .on_click(cx.listener(move |manager, _: &ClickEvent, _window, cx| {
                            manager.request_delete(id, cx);
                        })),
                )
                .child(
                    div()
                        .id(format!("workspace-drag-{id}"))
                        .w(px(28.))
                        .h(px(28.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_move()
                        .child(Icon::new(IconName::Menu))
                        .on_drag(WorkspaceDrag(id), |_drag, _position, _window, cx| {
                            cx.new(|_| WorkspaceDragPreview)
                        }),
                )
                                         });

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                TitleBar::new()
                    .border_color(gpui_kit::transparent_black())
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(app_titlebar_icon())
                            .child("Workspaces"),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .gap_4()
                    .p_4()
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_1()
                            .child(
                                SettingsWindow::icon_button(
                                    "open-command-center",
                                    "icons/layout-dashboard.svg",
                                    "Command Center",
                                    false,
                                )
                                .on_click(cx.listener(
                                    |manager, _: &ClickEvent, _window, cx| {
                                        CommandCenterWindow::open(
                                            Arc::clone(&manager.store),
                                            Arc::clone(&manager.messages),
                                            manager.settings.clone(),
                                            cx,
                                        );
                                    },
                                )),
                            )
                            .child(
                                Button::new("new-workspace")
                                    .icon(IconName::Plus)
                                    .primary()
                                    .tooltip("New workspace")
                                    .on_click(cx.listener(
                                        |manager, _: &ClickEvent, window, cx| {
                                            manager.open_workspace_dialog(None, window, cx);
                                        },
                                    )),
                            ),
                    )
                    .child(v_flex().gap_2().children(rows))
                    .children(self.error.as_ref().map(|error| div().child(error.clone())))
                    .children(self.show_workspace_dialog.then(|| {
                        div()
                            .absolute()
                            .inset_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(cx.theme().overlay)
                            // The keys ride on the overlay's own bubble-phase
                            // `on_key_down` rather than the gpui action route
                            // design.md holds in reserve, because the focused
                            // `Input` lets both through: its `Enter` handler
                            // calls `cx.propagate()` on a single-line field,
                            // and its `Escape` handler does the same unless
                            // `clean_on_escape` is set, which this input does
                            // not set. The listener lives here, not on the
                            // window, so it exists only in frames where the
                            // dialog is open.
                            .on_key_down(cx.listener(
                                move |manager, event: &gpui_kit::KeyDownEvent, window, cx| {
                                    match event.keystroke.key.as_str() {
                                        "enter" => {
                                            // Inert on a blank name, matching
                                            // the disabled confirm button
                                            // rather than raising the error
                                            // `save_name` would.
                                            if !name_is_blank {
                                                manager.confirm_workspace_dialog(window, cx);
                                            }
                                            cx.stop_propagation();
                                        },
                                        "escape" => {
                                            manager.cancel_workspace_dialog(window, cx);
                                            cx.stop_propagation();
                                        },
                                        _ => {},
                                    }
                                },
                            ))
                            .child(
                                v_flex()
                                    .w(px(360.))
                                    .gap_3()
                                    .p_4()
                                    .rounded(cx.theme().radius_lg)
                                    .bg(cx.theme().background)
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .child(div().text_lg().child(
                                        if self.workspace_dialog_id.is_some() {
                                            "Rename Workspace"
                                        } else {
                                            "New Workspace"
                                        },
                                    ))
                                    .child(Input::new(&self.name_input).h_full())
                                    .child(
                                        h_flex()
                                            .justify_end()
                                            .gap_2()
                                            .child(
                                                Button::new("cancel-workspace-dialog")
                                                    .label("Cancel")
                                                    .on_click(cx.listener(
                                                        |manager, _: &ClickEvent, window, cx| {
                                                            manager.cancel_workspace_dialog(
                                                                window, cx,
                                                            );
                                                        },
                                                    )),
                                            )
                                            .child(
                                                Button::new("confirm-workspace-dialog")
                                                    .label(if self.workspace_dialog_id.is_some() {
                                                        "Save"
                                                    } else {
                                                        "Create"
                                                    })
                                                    .primary()
                                                    .disabled(name_is_blank)
                                                    .on_click(cx.listener(
                                                        |manager, _: &ClickEvent, window, cx| {
                                                            manager.confirm_workspace_dialog(
                                                                window, cx,
                                                            );
                                                        },
                                                    )),
                                            ),
                                    ),
                            )
                    }))
                    .children(self.delete_workspace_id.map(|_| {
                        div()
                            .absolute()
                            .inset_0()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(cx.theme().overlay)
                            .child(
                                v_flex()
                                    .w(px(360.))
                                    .gap_3()
                                    .p_4()
                                    .rounded(cx.theme().radius_lg)
                                    .bg(cx.theme().background)
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .child(div().text_lg().child("Delete Workspace?"))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(format!(
                                                "Delete \"{}\" and its agents?",
                                                delete_name.as_deref().unwrap_or("this workspace")
                                            )),
                                    )
                                    .child(
                                        h_flex()
                                            .justify_end()
                                            .gap_2()
                                            .child(
                                                Button::new("cancel-delete-workspace")
                                                    .label("Cancel")
                                                    .on_click(cx.listener(
                                                        |manager, _: &ClickEvent, _window, cx| {
                                                            manager.cancel_delete(cx);
                                                        },
                                                    )),
                                            )
                                            .child(
                                                Button::new("confirm-delete-workspace")
                                                    .label("Delete")
                                                    .danger()
                                                    .on_click(cx.listener(
                                                        |manager, _: &ClickEvent, _window, cx| {
                                                            manager.confirm_delete(cx);
                                                        },
                                                    )),
                                            ),
                                    ),
                            )
                    })),
            )
            .children(crate::app_support::root_overlays(window, cx))
    }
}
