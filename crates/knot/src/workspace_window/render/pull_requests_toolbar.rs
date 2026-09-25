//! The Pull Requests view's toolbar: the search field, the status toggles,
//! the agent and sort pickers, and the list actions menu.
//!
//! Contract: "The view's search, filters and sort belong to the window" and
//! the requirements after it in the `pull-request-tracking` capability spec
//! under `openspec/specs/`.
//!
//! Drawn from groups the pane has already computed for the frame, so what a
//! toggle counts and what a bulk action removes are the rows on screen.

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::Selectable;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::Input;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::{App, ClickEvent, Context, Entity, IntoElement, ParentElement, Styled, Window, px};
use uuid::Uuid;

use crate::pull_request_filter::{self, PullRequestGroup, PullRequestSort, RowCategory};
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::pull_requests_actions::BulkRemoval;

/// The toolbar's search field width. Wide enough for the placeholder, which
/// says what the search matches.
const SEARCH_WIDTH: f32 = 300.;

/// One list action's menu entry: its label, and the status it narrows to,
/// or `None` for every shown row.
const REMOVALS: [(&str, Option<RowCategory>); 4] =
    [("pull_requests.remove_merged", Some(RowCategory::Merged)),
     ("pull_requests.remove_closed", Some(RowCategory::Closed)),
     ("pull_requests.remove_not_found", Some(RowCategory::NotFound)),
     ("pull_requests.remove_all", None)];

impl WorkspaceWindow {
    /// The toolbar, over `groups` (every row) and `shown` (the rows drawn).
    pub(super) fn pull_requests_toolbar(&mut self, groups: &[PullRequestGroup],
                                        shown: &[PullRequestGroup], window: &mut Window,
                                        cx: &mut Context<Self>)
                                        -> gpui_kit::AnyElement {
        let search = self.pull_request_search(window, cx);
        let filter = self.pull_request_filter(cx);
        let counts = pull_request_filter::category_counts(groups, &filter);
        let toggles = RowCategory::ALL.map(|category| {
                                          let count = counts.get(&category).copied().unwrap_or(0);
                                          let on = filter.statuses.contains(&category);
                                          status_toggle(category, count, on, cx)
                                      });
        let agents = self.pull_request_agent_choices(groups);
        let removal = |only| BulkRemoval { rows:     pull_request_filter::shown_rows(shown, only),
                                           filtered: filter.is_active(), };
        let removals = REMOVALS.map(|(key, only)| (key, removal(only)));
        let copied = pull_request_filter::copied_urls(shown);

        v_flex().gap_2()
                .child(h_flex().gap_2()
                               .items_center()
                               .child(search_field(&search))
                               .child(self.agent_picker(agents, cx))
                               .child(self.sort_picker(cx))
                               .child(actions_menu(removals, copied, cx)))
                .child(h_flex().gap_1().flex_wrap().children(toggles))
                .into_any_element()
    }

    /// Every agent heading a group, by name, for the agent picker.
    fn pull_request_agent_choices(&self, groups: &[PullRequestGroup]) -> Vec<(Uuid, String)> {
        let store = self.store.lock();
        let mut agents = pull_request_filter::listed_agents(groups).into_iter()
                                                                   .filter_map(|id| {
                                                                       store.agent(id)
                                                               .map(|agent| {
                                                                   (id, agent.name.clone())
                                                               })
                                                                   })
                                                                   .collect::<Vec<_>>();
        agents.sort_by_key(|(_, name)| name.to_lowercase());
        agents
    }

    fn agent_picker(&self, agents: Vec<(Uuid, String)>, cx: &mut Context<Self>)
                    -> impl IntoElement {
        let all = knot_core::l10n::t("pull_requests.agent_all");
        let current = self.pull_request_view
                          .agent
                          .and_then(|id| agents.iter().find(|(agent, _)| *agent == id))
                          .map_or_else(|| all.clone(), |(_, name)| name.clone());
        let entity = cx.entity();
        Button::new("pull-requests-agent-picker")
            .label(current)
            .dropdown_caret(true)
            .ghost()
            .small()
            .dropdown_menu(move |mut menu, _, _| {
                let choices = std::iter::once((None, all.clone()))
                    .chain(agents.iter().map(|(id, name)| (Some(*id), name.clone())));
                for (agent, label) in choices {
                    let entity = entity.clone();
                    menu = menu.item(PopupMenuItem::new(label).on_click(move |_, _, app| {
                                   entity.update(app, |view, cx| {
                                             view.pull_request_view.agent = agent;
                                             cx.notify();
                                         });
                               }));
                }
                menu
            })
    }

    fn sort_picker(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        Button::new("pull-requests-sort-picker")
            .label(self.pull_request_view.sort.to_string())
            .dropdown_caret(true)
            .ghost()
            .small()
            .dropdown_menu(move |mut menu, _, _| {
                for sort in PullRequestSort::ALL {
                    let entity = entity.clone();
                    menu = menu.item(PopupMenuItem::new(sort.to_string()).on_click(
                        move |_, _, app| {
                            entity.update(app, |view, cx| {
                                      view.pull_request_view.sort = sort;
                                      cx.notify();
                                  });
                        },
                    ));
                }
                menu
            })
    }
}

fn search_field(search: &Entity<gpui_kit::component::input::InputState>) -> impl IntoElement {
    Input::new(search).small()
                      .cleanable(true)
                      .w(px(SEARCH_WIDTH))
}

/// One status toggle: the status and how many rows it would show, drawn
/// selected while it narrows the list.
fn status_toggle(category: RowCategory, count: usize, on: bool,
                 cx: &mut Context<WorkspaceWindow>)
                 -> Button {
    let label = knot_core::l10n::t_with("pull_requests.filter_toggle",
                                        &[("label", &category.to_string()),
                                          ("count", &count.to_string())]);
    Button::new(gpui_kit::SharedString::from(format!("pull-requests-filter-{category:?}")))
        .label(label)
        .ghost()
        .small()
        .selected(on)
        .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                        view.toggle_pull_request_status(category);
                        cx.notify();
                    }))
}

/// The list actions menu. Each removal was computed over this frame's shown
/// rows, and is disabled when it would remove nothing.
fn actions_menu(removals: [(&'static str, BulkRemoval); 4], copied: String,
                cx: &mut Context<WorkspaceWindow>)
                -> impl IntoElement {
    let entity = cx.entity();
    let removals = removals.map(|(key, removal)| (key, removal.rows, removal.filtered));
    crate::controls::icon_button("pull-requests-actions",
                                 "icons/ellipsis.svg",
                                 knot_core::l10n::t("pull_requests.actions"),
                                 false).dropdown_menu(move |mut menu, _, _| {
        let refresh = entity.clone();
        menu = menu.item(PopupMenuItem::new(knot_core::l10n::t("pull_requests.refresh_now"))
                                 .on_click(move |_, _, app| {
                                     refresh.update(app, |view, cx| {
                                                view.refresh_pull_requests_now();
                                                cx.notify();
                                            });
                                 }));
        let text = copied.clone();
        menu = menu.item(PopupMenuItem::new(knot_core::l10n::t("pull_requests.copy_urls"))
                                 .disabled(text.is_empty())
                                 .on_click(move |_, _, app: &mut App| {
                                     WorkspaceWindow::copy_to_clipboard(text.clone(), app);
                                 }));
        menu = menu.item(PopupMenuItem::separator());
        for (key, rows, filtered) in &removals {
            let entity = entity.clone();
            let rows = rows.clone();
            let filtered = *filtered;
            menu = menu.item(PopupMenuItem::new(knot_core::l10n::t(key))
                                     .disabled(rows.is_empty())
                                     .on_click(move |_, window, app| {
                                         // Deferred: the menu dismisses itself
                                         // after this handler, and would take
                                         // a dialog opened inline down with it.
                                         let entity = entity.clone();
                                         let removal = BulkRemoval { rows: rows.clone(),
                                                                     filtered };
                                         window.defer(app, move |window, app| {
                                                   entity.update(app, |view, cx| {
                                                             view.confirm_bulk_remove(removal,
                                                                                      window,
                                                                                      cx);
                                                         });
                                               });
                                     }));
        }
        menu
    })
}
