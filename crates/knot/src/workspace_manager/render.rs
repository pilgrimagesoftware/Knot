//! What the workspace manager draws.
//!
//! Split from the state it draws: the window's `Render` was 249 lines, one
//! expression building the title bar, the toolbar, a row per workspace and
//! the name dialog, so nothing in it could be read without reading all of
//! it. Each of those is a method here, and `render` is the shape of the
//! window. The name dialog has since left entirely - it is built by
//! `dialog` and drawn by the dialog layer gpui-component's `Root` plugin
//! mounts in every window.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::Bounds;
use gpui_kit::ClickEvent;
use gpui_kit::Context;
use gpui_kit::Div;
use gpui_kit::DragMoveEvent;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Pixels;
use gpui_kit::Render;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::assets::IconName;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::TitleBar;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::div;
use gpui_kit::px;

use super::drag::DropEdge;
use super::drag::drop_line as drop_line_element;
use super::drag::drop_line_edge;
use crate::app_support::app_titlebar_icon;
use crate::command_center::CommandCenterWindow;
use crate::workspace_manager::WorkspaceDrag;
use crate::workspace_manager::WorkspaceDragPreview;
use crate::workspace_manager::WorkspaceManager;

/// What one row is drawn from, gathered by `render` so the row itself
/// needs no access to the manager's state.
struct RowFacts {
    workspace:  knot_core::Workspace,
    /// Its place in the order, which every drop gap is counted in.
    row:        usize,
    selected:   bool,
    /// Where the drop line goes on this row, if the pending drop lands at
    /// one of its edges.
    drop_line:  Option<DropEdge>,
    row_bounds: Rc<RefCell<Vec<Bounds<Pixels>>>>,
}

/// The name and agent count a row leads with - shared with the drag
/// preview, which is a copy of the row.
pub(super) fn row_summary(name: String, agents: String, cx: &App) -> impl IntoElement + use<> {
    v_flex().flex_1()
            .min_w_0()
            .gap_1()
            .child(div().text_lg().child(name))
            .child(div().text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(agents))
}

/// The grip a row is dragged by - shared with the drag preview.
pub(super) fn drag_handle() -> Div {
    div().w(px(28.))
         .h(px(28.))
         .flex()
         .flex_shrink_0()
         .items_center()
         .justify_center()
         .child(Icon::new(IconName::Menu))
}

impl WorkspaceManager {
    /// One workspace's row: its name and agent count, the three actions it
    /// offers, and the handle it is dragged by.
    /// `use<>` because the row borrows nothing it is given: it copies what
    /// it draws out of the workspace and registers its listeners against
    /// the context rather than holding it, which is what lets the caller
    /// build every row from one `&mut Context`.
    fn workspace_row(facts: RowFacts, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let RowFacts { workspace,
                       row,
                       selected,
                       drop_line,
                       row_bounds, } = facts;
        let id = workspace.id;
        let agents = knot_core::l10n::pluralize(workspace.agent_ids.len() as u64,
                                                "count.agent",
                                                "count.agents");
        let manager = cx.entity().downgrade();
        let preview_name = workspace.name.clone();
        let preview_agents = agents.clone();
        h_flex()
                .id(format!("workspace-row-{id}"))
                .debug_selector(move || format!("workspace-row-{row}"))
                .relative()
                .on_drag_move(
                    cx.listener(move |manager, event: &DragMoveEvent<WorkspaceDrag>, _window, cx| {
                        manager.drag_moved(row, event, cx);
                    }),
                )
                .on_drop(
                    cx.listener(move |manager, drag: &WorkspaceDrag, _window, cx| {
                        manager.dropped(drag, row, cx);
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
                // Hidden once the drag ends, whether or not it ended in a
                // drop: a release outside every row leaves the target set.
                .children(drop_line.filter(|_| cx.has_active_drag())
                                   .map(|edge| drop_line_element(edge, cx)))
                .child(row_summary(workspace.name, agents, cx))
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
                        // A text field with an I-beam: renaming is editing
                        // the name, which a document icon did not say.
                        // Not in `IconName`; `AllAssets` embeds it.
                        .icon(Icon::default().path("icons/text-cursor-input.svg"))
                        .ghost()
                        .tooltip(knot_core::l10n::t("workspace_manager.rename"))
                        .on_click(cx.listener(move |manager, _: &ClickEvent, window, cx| {
                            manager.open_workspace_dialog(Some(id), window, cx);
                        })),
                )
                .child(
                    Button::new(format!("delete-workspace-{id}"))
                        // Ghost like its neighbours, with only the icon red:
                        // `.danger()` is a variant that would replace
                        // `.ghost()` and fill the button.
                        .icon(Icon::new(IconName::Trash).text_color(cx.theme().danger))
                        .ghost()
                        .tooltip(knot_core::l10n::t("workspace_manager.delete"))
                        .on_click(cx.listener(move |manager, _: &ClickEvent, window, cx| {
                            manager.request_delete(id, window, cx);
                        })),
                )
                .child(
                    drag_handle()
                        .id(format!("workspace-drag-{id}"))
                        .debug_selector(move || format!("workspace-drag-handle-{row}"))
                        .cursor_move()
                        .on_drag(WorkspaceDrag { id, row }, move |_drag, cursor_offset, window, cx| {
                            // A target left over from a drag released
                            // outside every row must not draw during this one.
                            manager.update(cx, |manager, _| manager.drag.target = None).ok();
                            let handle = window.mouse_position() - cursor_offset;
                            let bounds = row_bounds.borrow().get(row).copied();
                            let preview = WorkspaceDragPreview::new(preview_name.clone(),
                                                                    preview_agents.clone(),
                                                                    bounds,
                                                                    handle);
                            cx.new(|_| preview)
                        }),
                )
    }
}

impl Render for WorkspaceManager {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // One lock for both reads: the current workspace was being re-read
        // per row, taking the store lock once per workspace in the middle of
        // building the element tree.
        let (workspaces, current_workspace) = {
            let store = self.store.lock();
            (store.workspaces().to_vec(), store.current_workspace_id())
        };
        // A loop rather than a `map`: each row registers listeners, which
        // needs the context mutably, and a closure holding it across the
        // iteration cannot.
        let count = workspaces.len();
        let gap = self.drag.target.map(|target| target.gap);
        let mut rows = Vec::with_capacity(count);
        for (row, workspace) in workspaces.into_iter().enumerate() {
            let facts =
                RowFacts { selected: current_workspace == Some(workspace.id),
                           drop_line: gap.and_then(|gap| drop_line_edge(row, count, gap)),
                           row_bounds: Rc::clone(&self.drag.row_bounds),
                           workspace,
                           row };
            rows.push(Self::workspace_row(facts, cx));
        }
        let row_bounds = Rc::clone(&self.drag.row_bounds);

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
                    .min_h_0()
                    .gap_4()
                    .p_4()
                    .child(
                        h_flex()
                            .justify_end()
                            .gap_1()
                            .child(
                                crate::controls::icon_button(
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
                    // The roster grows with every workspace, so it scrolls
                    // rather than running past the window; the toolbar above
                    // it stays put. `min_h_0` here and on the parent is what
                    // lets it shrink below its content's height at all.
                    //
                    // The selector is on this viewport, not on the rows'
                    // column: the scrollbar wrapper makes that column the
                    // scrolled content, which is as tall as every row.
                    .child(div().debug_selector(|| "workspace-manager-list".into())
                                .flex_1()
                                .min_h_0()
                                .child(v_flex().on_children_prepainted(move |bounds, _window, _cx| {
                                                   *row_bounds.borrow_mut() = bounds;
                                               })
                                               .id("workspace-manager-list")
                                               .size_full()
                                               .gap_2()
                                               // Room above the first row and
                                               // below the last for the drop
                                               // line, which sits outside its
                                               // row and would be clipped.
                                               .py(px(6.))
                                               .overflow_y_scrollbar()
                                               .children(rows)))
                    .children(self.error.as_ref().map(|error| div().child(error.clone())))
            )
    }
}
