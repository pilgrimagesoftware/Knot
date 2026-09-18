//! Shared agent-card grid used by both the workspace-scoped in-place
//! dashboard (`WorkspaceWindow`) and the global `CommandCenterWindow`.
//! See `openspec/changes/dashboard-view/design.md`.

use gpui_kit::base::{StyledExt, h_flex, v_flex};
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::{
    ClickEvent, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    div, hsla, px, rgb,
};
use uuid::Uuid;

use crate::{state_color, state_label};

pub(crate) const CARD_WIDTH: f32 = 280.;
/// Every agent card is this tall, whatever it has to show. Two of a card's
/// four rows are conditional (the status line, the diff stat), so sizing to
/// content left neighbouring cards visibly uneven; a fixed height keeps the
/// grid uniform and the rows top-aligned within it.
pub(crate) const CARD_HEIGHT: f32 = 124.;
pub(crate) const GRID_SPACING: f32 = 16.;

/// One agent's data as needed by a dashboard card - a plain snapshot, not a
/// reference into the live `AgentStore`, so the render function has no
/// borrow-lifetime tie to the caller's lock.
#[derive(Debug, Clone)]
pub(crate) struct DashboardAgent {
    pub id:           Uuid,
    pub avatar:       String,
    pub name:         String,
    pub folder_name:  String,
    pub state:        knot_agents::AgentState,
    pub is_shell:     bool,
    pub header_title: String,
    pub git_stats:    Option<knot_git::DiffStats>,
}

/// One workspace's section as needed by a dashboard grid.
#[derive(Debug, Clone)]
pub(crate) struct DashboardWorkspace {
    pub id:        Uuid,
    pub name:      String,
    pub color_hex: String,
    pub agents:    Vec<DashboardAgent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum DashboardSort {
    #[default]
    Manual,
    Name,
    Status,
}

impl DashboardSort {
    pub fn label(self) -> String {
        let key = match self {
            DashboardSort::Manual => "dashboard.sort.manual",
            DashboardSort::Name => "dashboard.sort.name",
            DashboardSort::Status => "dashboard.sort.status",
        };
        knot_core::l10n::t(key)
    }

    /// Rank used for `Status` sort - agents needing attention float first.
    fn status_rank(state: knot_agents::AgentState) -> u8 {
        match state {
            knot_agents::AgentState::Error => 0,
            knot_agents::AgentState::Input => 1,
            knot_agents::AgentState::Running => 2,
            knot_agents::AgentState::Idle => 3,
        }
    }

    pub fn sorted(self, mut agents: Vec<DashboardAgent>) -> Vec<DashboardAgent> {
        match self {
            DashboardSort::Manual => agents,
            DashboardSort::Name => {
                agents.sort_by_key(|a| a.name.to_lowercase());
                agents
            }
            DashboardSort::Status => {
                agents.sort_by_key(|agent| Self::status_rank(agent.state));
                agents
            }
        }
    }
}

/// A dropdown button cycling through `DashboardSort` variants.
pub(crate) fn sort_picker(current: DashboardSort,
                          on_select: impl Fn(DashboardSort,
                             &mut gpui_kit::Window,
                             &mut gpui_kit::App)
                          + Clone
                          + 'static)
                          -> impl IntoElement {
    Button::new("dashboard-sort-picker")
        .label(current.label())
        .dropdown_caret(true)
        .ghost()
        .small()
        .dropdown_menu(move |mut menu, _, _| {
            for sort in [
                DashboardSort::Manual,
                DashboardSort::Name,
                DashboardSort::Status,
            ] {
                let on_select = on_select.clone();
                menu = menu.item(PopupMenuItem::new(sort.label()).on_click(
                    move |_, window, cx| {
                        on_select(sort, window, cx);
                    },
                ));
            }
            menu
        })
}

/// One agent card: avatar, name, status, folder name, and git diff stats.
fn agent_card(agent: &DashboardAgent, muted: gpui_kit::Hsla,
              on_tap: impl Fn(Uuid, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static)
              -> impl IntoElement {
    let id = agent.id;
    let avatar = agent.avatar.clone();
    let name = agent.name.clone();
    let folder_name = agent.folder_name.clone();
    let header_title = agent.header_title.clone();
    let is_shell = agent.is_shell;
    let state = agent.state;
    let git_stats = agent.git_stats;

    // A plain clickable container, not `Button` - `Button`'s "Normal Button"
    // sizing forces a fixed `h_8` height regardless of content unless an
    // explicit height is set, which clips/truncates a multi-line card.
    v_flex().id(gpui_kit::ElementId::from(format!("dashboard-agent-card-{id}")))
            .cursor_pointer()
            .w(px(CARD_WIDTH))
            .h(px(CARD_HEIGHT))
            .overflow_hidden()
            .justify_start()
            .gap_2()
            .p_3()
            .rounded_lg()
            .border_1()
            .border_color(hsla(0., 0., 0.5, 0.12))
            .bg(hsla(0., 0., 0.5, 0.04))
            .child(h_flex().w_full()
                           .gap_2()
                           .items_start()
                           .child(div().text_xl().flex_shrink_0().child(avatar))
                           .child(v_flex().flex_1()
                                          .min_w_0()
                                          .gap_0p5()
                                          .child(div().font_semibold().child(name))
                                          .child(div().text_sm()
                                                      .text_color(rgb(0x888888))
                                                      .overflow_hidden()
                                                      .whitespace_nowrap()
                                                      .text_ellipsis()
                                                      .child(folder_name)))
                           .children((!is_shell).then(|| {
                                                    div().text_sm()
                                                         .flex_shrink_0()
                                                         .text_color(state_color(state))
                                                         .child(state_label(state))
                                                })))
            .children((!header_title.is_empty()).then(|| {
                                                    div().text_sm()
                                                         .overflow_hidden()
                                                         .whitespace_nowrap()
                                                         .text_ellipsis()
                                                         .child(header_title)
                                                }))
            .children(git_stats.filter(|stats| stats.files_changed > 0)
                               .map(|stats| {
                                   crate::app_state::diff_stats_row(&stats, muted).text_xs()
                               }))
            .on_click(move |_: &ClickEvent, window, cx| on_tap(id, window, cx))
}

/// The "Add Agent" tile, styled to match the card grid it sits in.
fn add_agent_tile(workspace_id: Uuid,
                  on_add_agent: impl Fn(Uuid, &mut gpui_kit::Window, &mut gpui_kit::App) + 'static)
                  -> impl IntoElement {
    h_flex().id(gpui_kit::ElementId::from(format!("dashboard-add-agent-{workspace_id}")))
            .cursor_pointer()
            .w(px(CARD_WIDTH))
            .h(px(CARD_HEIGHT))
            .gap_2()
            .items_center()
            .justify_center()
            .rounded_lg()
            .border_1()
            .border_color(hsla(0., 0., 0.5, 0.12))
            .child(knot_core::l10n::t("dashboard.add_agent"))
            .on_click(move |_: &ClickEvent, window, cx| on_add_agent(workspace_id, window, cx))
}

/// One workspace's section: color bar + name (+ nav when global) + agent
/// grid, or an "No agents" empty state.
pub(crate) fn workspace_section(workspace: DashboardWorkspace, is_global: bool,
                                muted: gpui_kit::Hsla,
                                on_agent_tap: impl Fn(Uuid,
                                   &mut gpui_kit::Window,
                                   &mut gpui_kit::App)
                                + Clone
                                + 'static,
                                on_workspace_nav: impl Fn(Uuid,
                                   &mut gpui_kit::Window,
                                   &mut gpui_kit::App)
                                + Clone
                                + 'static,
                                on_add_agent: impl Fn(Uuid,
                                   &mut gpui_kit::Window,
                                   &mut gpui_kit::App)
                                + Clone
                                + 'static)
                                -> impl IntoElement {
    let color: gpui_kit::Hsla =
        gpui_kit::Rgba::try_from(workspace.color_hex.as_str()).map(Into::into)
                                                              .unwrap_or_else(|_| {
                                                                  rgb(0x1B4FB2).into()
                                                              });

    let title: gpui_kit::AnyElement = if is_global {
        let workspace_id = workspace.id;
        Button::new(gpui_kit::ElementId::from(format!(
            "dashboard-workspace-nav-{workspace_id}"
        )))
        .ghost()
        .label(workspace.name.clone())
        .on_click(move |_: &ClickEvent, window, cx| on_workspace_nav(workspace_id, window, cx))
        .into_any_element()
    }
    else {
        div().font_semibold()
             .text_lg()
             .child(workspace.name.clone())
             .into_any_element()
    };

    v_flex().w_full()
            .gap_3()
            .child(h_flex().gap_2p5()
                           .items_center()
                           .child(div().w(px(4.)).h(px(24.)).rounded(px(3.)).bg(color))
                           .child(title))
            .child(if workspace.agents.is_empty() {
                       div().pl_3()
                            .text_sm()
                            .text_color(rgb(0x888888))
                            .child(knot_core::l10n::t("dashboard.no_agents"))
                            .into_any_element()
                   }
                   else {
                       h_flex().w_full()
                               .flex_wrap()
                               .gap(px(GRID_SPACING))
                               .children(workspace.agents.iter().map(|agent| {
                                                                    agent_card(agent, muted, {
                                                                        let on_agent_tap =
                                                                            on_agent_tap.clone();
                                                                        move |id, window, cx| {
                                                                            on_agent_tap(id,
                                                                                         window, cx)
                                                                        }
                                                                    })
                                                                }))
                               .child(add_agent_tile(workspace.id, {
                                          let on_add_agent = on_add_agent.clone();
                                          move |id, window, cx| on_add_agent(id, window, cx)
                                      }))
                               .into_any_element()
                   })
}
