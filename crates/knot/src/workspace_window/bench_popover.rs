//! The bench popover on the New Agent button - the port of the Swift app's
//! `BenchDropdownView` (`Skwad/Views/Sidebar/BenchSectionView.swift`).
//!
//! Contract: `openspec/specs/agent-list-ui/spec.md`, "The New Agent button
//! opens the bench".
//!
//! The chevron beside the New Agent button opens it: Create New Agent, the
//! BENCH header, then the entries - click to deploy into this workspace, or
//! remove after a confirmation. The bench is read from the live settings
//! surface each time the popover draws, so an entry saved from another
//! window or from Settings is listed; that is an in-memory read, not I/O.
//!
//! Two deliberate differences from the Swift view: the bench is read rather
//! than observed, and each entry's remove control is always present (muted
//! until the row is hovered) so it can be reached by keyboard - the Swift
//! view shows it on hover only.

use gpui_kit::App;
use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::SharedString;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::assets::IconName;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::popover::Popover;
use gpui_kit::div;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::px;
use knot_core::BenchAgent;
use uuid::Uuid;

use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::menus::confirm_then;

/// One entry as the popover draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BenchRow {
    pub(super) id:          Uuid,
    pub(super) avatar:      String,
    pub(super) name:        String,
    /// The folder's last path component; the full path would not fit.
    pub(super) folder_name: String,
}

/// The popover's rows for `bench`, in bench order.
pub(super) fn bench_rows(bench: &[BenchAgent]) -> Vec<BenchRow> {
    bench.iter()
         .map(|entry| BenchRow { id:          entry.id,
                                 avatar:      entry.avatar.clone(),
                                 name:        entry.name.clone(),
                                 folder_name:
                                     knot_core::folder_name(&entry.folder).unwrap_or_else(|| {
                                                                              entry.folder.clone()
                                                                          }), })
         .collect()
}

impl WorkspaceWindow {
    /// The chevron that opens the bench popover, beside the New Agent
    /// button. It keeps its place in the compact sidebar, where its meaning
    /// is carried by its tooltip either way.
    pub(super) fn bench_popover(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let entity = cx.entity();
        let label = knot_core::l10n::t("sidebar.bench_popover.open");
        let trigger = Button::new("workspace-new-agent-bench").icon(IconName::ChevronDown)
                                                              .ghost()
                                                              .accessibility_label(label.clone())
                                                              .tooltip(label);
        Popover::new("workspace-bench-popover").anchor(gpui_kit::Anchor::BottomLeft)
                                               .trigger(trigger)
                                               .open(self.bench_popover_open)
                                               .on_open_change({
                                                   let entity = entity.clone();
                                                   move |open, _, app| {
                                                       entity.update(app, |view, cx| {
                                                                 view.bench_popover_open = *open;
                                                                 cx.notify();
                                                             });
                                                   }
                                               })
                                               .content(move |_, _, app| {
                                                   bench_popover_content(&entity, app)
                                               })
    }

    /// Remove bench entry `id` and leave the popover open on the updated
    /// list - the Settings window's Bench tab removes through the same
    /// settings helper.
    pub(super) fn remove_bench_entry(&mut self, id: Uuid, cx: &App) {
        if let Err(error) =
            crate::settings_global::write_persisting(cx, |settings| settings.remove_bench_agent(id))
        {
            eprintln!("failed to remove the bench entry: {error}");
        }
        self.bench_popover_open = true;
    }
}

/// What the open popover draws: Create New Agent, a divider, the BENCH
/// header, then the entries or the empty-bench hint.
fn bench_popover_content(entity: &gpui_kit::Entity<WorkspaceWindow>, app: &mut App)
                         -> impl IntoElement + use<> {
    let bench = crate::settings_global::read(app).bench_agents.clone();
    let hovered = entity.read(app).bench_popover_hovered;
    let muted = app.theme().muted_foreground;
    let create = {
        let entity = entity.clone();
        Button::new("bench-popover-create").icon(IconName::Plus)
                                           .label(knot_core::l10n::t("sidebar.bench_popover.create"))
                                           .ghost()
                                           .w_full()
                                           .on_click(move |_, _, app| {
                                               entity.update(app, |view, cx| {
                                                         view.bench_popover_open = false;
                                                         view.open_new_agent_dialog(cx);
                                                         cx.notify();
                                                     });
                                           })
    };
    let header = div().px_2()
                      .text_xs()
                      .text_color(muted)
                      .child(knot_core::l10n::t("sidebar.bench_popover.header"));
    let rows = bench_rows(&bench);
    let list = if rows.is_empty() {
        div().px_2()
             .py_2()
             .text_xs()
             .text_color(muted)
             .child(knot_core::l10n::t("sidebar.bench_popover.empty"))
             .into_any_element()
    }
    else {
        div().id("bench-popover-list")
             .max_h(px(300.))
             .overflow_y_scroll()
             .child(v_flex().children(rows.into_iter().map(|row| {
                                                          let entry =
                                                              bench.iter()
                                                                   .find(|entry| entry.id == row.id)
                                                                   .cloned();
                                                          bench_row(entity, row, entry, hovered,
                                                                    app)
                                                      })))
             .into_any_element()
    };
    v_flex().w(px(260.))
            .gap_1()
            .child(create)
            .child(div().h(px(1.)).bg(app.theme().border))
            .child(header)
            .child(list)
}

/// One entry: the deploy target (avatar, name, folder name) and its remove
/// control, muted until the row is hovered.
fn bench_row(entity: &gpui_kit::Entity<WorkspaceWindow>, row: BenchRow,
             entry: Option<BenchAgent>, hovered: Option<Uuid>, app: &App)
             -> impl IntoElement + use<> {
    let id = row.id;
    let is_hovered = hovered == Some(id);
    let theme = app.theme();
    let (muted, highlight) = (theme.muted_foreground, theme.accent);
    let deploy = {
        let entity = entity.clone();
        Button::new(SharedString::from(format!("bench-entry-{id}")))
            .accessibility_label(row.name.clone())
            .ghost()
            .flex_1()
            .child(h_flex().gap_2()
                           .min_w_0()
                           .child(div().text_lg().child(row.avatar.clone()))
                           .child(v_flex().min_w_0()
                                          .child(div().truncate().child(row.name.clone()))
                                          .child(div().truncate()
                                                      .text_xs()
                                                      .text_color(muted)
                                                      .child(row.folder_name.clone()))))
            .on_click(move |_, _, app| {
                let Some(entry) = entry.clone()
                else {
                    return;
                };
                entity.update(app, |view, cx| {
                          view.bench_popover_open = false;
                          view.deploy_bench_entry(&entry, cx);
                          cx.notify();
                      });
            })
    };
    let remove = {
        let entity = entity.clone();
        let name = row.name.clone();
        let label = knot_core::l10n::t_with("sidebar.bench_popover.remove", &[("name", &name)]);
        Button::new(SharedString::from(format!("bench-entry-remove-{id}")))
            .icon(IconName::Trash)
            .ghost()
            .small()
            .accessibility_label(label.clone())
            .tooltip(knot_core::l10n::t("sidebar.bench_popover.remove_tooltip"))
            .when(!is_hovered, |button| button.text_color(muted))
            .on_click(move |_, window, app| {
                let entity = entity.clone();
                let body = knot_core::l10n::t_with("sidebar.bench_popover.confirm_remove_body",
                                                   &[("name", &name)]);
                confirm_then(window,
                             app,
                             knot_core::l10n::t("sidebar.bench_popover.confirm_remove_title"),
                             body,
                             move |app| {
                                 entity.update(app, |view, cx| {
                                           view.remove_bench_entry(id, cx);
                                           cx.notify();
                                       });
                             });
            })
    };
    let hover_entity = entity.clone();
    h_flex().id(SharedString::from(format!("bench-row-{id}")))
            .gap_1()
            .rounded_md()
            .when(is_hovered, |row| row.bg(highlight))
            .on_hover(move |hovering, _, app| {
                let hovering = *hovering;
                hover_entity.update(app, |view, cx| {
                                if hovering {
                                    view.bench_popover_hovered = Some(id);
                                }
                                else if view.bench_popover_hovered == Some(id) {
                                    view.bench_popover_hovered = None;
                                }
                                cx.notify();
                            });
            })
            .child(deploy)
            .child(remove)
}

#[cfg(test)]
mod tests;
