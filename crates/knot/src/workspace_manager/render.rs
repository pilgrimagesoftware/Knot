//! What the workspace manager draws.
//!
//! Split from the state it draws: the window's `Render` was 249 lines, one
//! expression building the title bar, the toolbar, a row per workspace and
//! the name dialog, so nothing in it could be read without reading all of
//! it. Each of those is a method here, and `render` is the shape of the
//! window.

use std::sync::Arc;

use gpui_kit::AppContext;
use gpui_kit::ClickEvent;
use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::assets::IconName;
use gpui_kit::base::Disableable;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::TitleBar;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::Input;
use gpui_kit::div;
use gpui_kit::px;

use crate::app_support::app_titlebar_icon;
use crate::command_center::CommandCenterWindow;
use crate::settings_window::SettingsWindow;
use crate::workspace_manager::WorkspaceDrag;
use crate::workspace_manager::WorkspaceDragPreview;
use crate::workspace_manager::WorkspaceManager;
use crate::workspace_manager::workspace_name_is_blank;

impl Render for WorkspaceDragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().p_2()
             .child(knot_core::l10n::t("workspace_manager.workspace"))
    }
}

impl WorkspaceManager {
    /// One workspace's row: its name and agent count, the three actions it
    /// offers, and the handle it is dragged by.
    /// `use<>` because the row borrows nothing it is given: it copies what
    /// it draws out of the workspace and registers its listeners against
    /// the context rather than holding it, which is what lets the caller
    /// build every row from one `&mut Context`.
    fn workspace_row(workspace: knot_core::Workspace, selected: bool, cx: &mut Context<Self>)
                     -> impl IntoElement + use<> {
        let id = workspace.id;
        let agents = knot_core::l10n::pluralize(workspace.agent_ids.len() as u64,
                                                "count.agent",
                                                "count.agents");
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
                        .child(div().text_lg().child(workspace.name))
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(agents),
                        ),
                )
                .child(
                    Button::new(format!("open-workspace-{id}"))
                        .icon(IconName::ExternalLink)
                        .ghost()
                        .tooltip(knot_core::l10n::t("workspace_manager.open"))
                        .on_click(cx.listener(move |manager, _: &ClickEvent, _window, cx| {
                            manager.open(id, cx);
                        })),
                )
                .child(
                    Button::new(format!("rename-workspace-{id}"))
                        .icon(IconName::FileText)
                        .ghost()
                        .tooltip(knot_core::l10n::t("workspace_manager.rename"))
                        .on_click(cx.listener(move |manager, _: &ClickEvent, window, cx| {
                            manager.open_workspace_dialog(Some(id), window, cx);
                        })),
                )
                .child(
                    Button::new(format!("delete-workspace-{id}"))
                        .icon(IconName::Delete)
                        .danger()
                        .tooltip(knot_core::l10n::t("workspace_manager.delete"))
                        .on_click(cx.listener(move |manager, _: &ClickEvent, window, cx| {
                            manager.request_delete(id, window, cx);
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
    }

    /// The create/rename dialog, or `None` when it is not open.
    ///
    /// Takes `name_is_blank` rather than reading the input again: the
    /// disabled confirm button and the Return key must agree about what an
    /// empty name is, and they do by being the same value.
    fn name_dialog(&self, name_is_blank: bool, cx: &mut Context<Self>)
                   -> Option<impl IntoElement + use<>> {
        let renaming = self.workspace_dialog_id.is_some();
        self.show_workspace_dialog.then(|| {
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
                                    .child(div().text_lg().child(knot_core::l10n::t(
                                        if renaming {
                                            "workspace_manager.rename_title"
                                        } else {
                                            "workspace_manager.new_title"
                                        },
                                    )))
                                    .child(Input::new(&self.name_input).h_full())
                                    .child(
                                        h_flex()
                                            .justify_end()
                                            .gap_2()
                                            .child(
                                                Button::new("cancel-workspace-dialog")
                                                    .label(knot_core::l10n::t("workspace_manager.cancel"))
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
                                                    .label(knot_core::l10n::t(if renaming {
                                                        "workspace_manager.save"
                                                    } else {
                                                        "workspace_manager.create"
                                                    }))
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
        })
    }
}

impl Render for WorkspaceManager {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // One lock for both reads: the current workspace was being re-read
        // per row, taking the store lock once per workspace in the middle of
        // building the element tree.
        let (workspaces, current_workspace) = {
            let store = self.store.lock();
            (store.workspaces().to_vec(), store.current_workspace_id())
        };
        let name_is_blank = workspace_name_is_blank(&self.name_input.read(cx).value());
        // A loop rather than a `map`: each row registers listeners, which
        // needs the context mutably, and a closure holding it across the
        // iteration cannot.
        let mut rows = Vec::with_capacity(workspaces.len());
        for workspace in workspaces {
            let selected = current_workspace == Some(workspace.id);
            rows.push(Self::workspace_row(workspace, selected, cx));
        }

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
                            .child(knot_core::l10n::t("workspace_manager.title")),
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
                                    knot_core::l10n::t("dashboard.command_center"),
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
                                    .tooltip(knot_core::l10n::t("workspace_manager.new"))
                                    .on_click(cx.listener(
                                        |manager, _: &ClickEvent, window, cx| {
                                            manager.open_workspace_dialog(None, window, cx);
                                        },
                                    )),
                            ),
                    )
                    .child(v_flex().gap_2().children(rows))
                    .children(self.error.as_ref().map(|error| div().child(error.clone())))
                    .children(self.name_dialog(name_is_blank, cx))
            )
            .children(crate::app_support::root_overlays(window, cx))
    }
}
