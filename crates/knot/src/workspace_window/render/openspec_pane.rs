//! The Changes view's OpenSpec tab (#504): the workspace's repositories'
//! un-archived OpenSpec changes, grouped by repository.
//!
//! Contract: `openspec/changes/pr-view-issues-and-changes/specs/
//! workspace-openspec-changes/spec.md`.

use std::path::PathBuf;

use gpui_kit::base::{StyledExt, h_flex, v_flex};
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::Button;
use gpui_kit::component::menu::{ContextMenuExt, PopupMenu, PopupMenuItem};
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::{
    App, ClickEvent, Context, Entity, InteractiveElement, IntoElement, ParentElement, SharedString,
    Styled, Window, div,
};

use super::row_actions::row_actions_button;
use super::send_prompt_menu::{SendTarget, WorkItemRef, with_send_prompt};
use crate::openspec_changes::ChangeEntry;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::openspec_view::{ChangeGroup, OpenSpecMessage};

impl WorkspaceWindow {
    /// The OpenSpec tab's body.
    pub(super) fn openspec_body(&mut self, window: &mut Window, cx: &mut Context<Self>)
                                -> gpui_kit::AnyElement {
        let frame = self.openspec_frame(cx);
        let toolbar = self.openspec_toolbar(frame.headings, window, cx);
        let targets = self.prompt_targets();
        let entity = cx.entity();
        div().id("workspace-openspec")
             .flex_1()
             .min_h_0()
             .overflow_y_scrollbar()
             .child(v_flex().min_h_full()
                            .gap_4()
                            .p_5()
                            .child(toolbar)
                            .children(frame.message.map(|message| render_message(message, cx)))
                            .children(frame.shown
                                           .into_iter()
                                           .map(|group| {
                                               render_group(group, &targets, &entity, cx)
                                           })))
             .into_any_element()
    }
}

fn render_message(message: OpenSpecMessage, cx: &mut Context<WorkspaceWindow>)
                  -> gpui_kit::AnyElement {
    let text = knot_core::l10n::t(match message {
                                      OpenSpecMessage::Checking => "openspec_changes.checking",
                                      OpenSpecMessage::NoChanges => "openspec_changes.empty",
                                      OpenSpecMessage::NoneMatch => "openspec_changes.no_match",
                                  });
    v_flex().gap_2()
            .items_start()
            .child(div().text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(text))
            .children((message == OpenSpecMessage::NoneMatch).then(|| {
                          Button::new("openspec-clear-filters")
                    .label(knot_core::l10n::t("changes_view.clear_filters"))
                    .on_click(cx.listener(|view, _: &ClickEvent, window, cx| {
                                    view.clear_openspec_filters(window, cx);
                                }))
                      }))
            .into_any_element()
}

fn render_group(group: ChangeGroup, targets: &[SendTarget], entity: &Entity<WorkspaceWindow>,
                cx: &mut Context<WorkspaceWindow>)
                -> gpui_kit::AnyElement {
    v_flex().gap_2()
            .child(div().text_sm().font_semibold().child(group.heading))
            .children(group.changes
                           .into_iter()
                           .map(|change| render_change(change, targets, entity, cx)))
            .into_any_element()
}

fn render_change(change: ChangeEntry, targets: &[SendTarget], entity: &Entity<WorkspaceWindow>,
                 cx: &mut Context<WorkspaceWindow>)
                 -> gpui_kit::AnyElement {
    let (name, dir) = change_actions(&change);
    let menu = ChangeMenu { name,
                            dir,
                            targets: targets.to_vec(),
                            entity: entity.clone() };
    let context = menu.clone();
    h_flex().id(SharedString::from(format!("openspec-change-{}", change.dir.display())))
            .w_full()
            .min_w_0()
            .gap_3()
            .items_center()
            .p_2()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .child(v_flex().flex_1()
                           .min_w_0()
                           .child(div().overflow_hidden()
                                       .whitespace_nowrap()
                                       .text_ellipsis()
                                       .font_family(cx.theme().mono_font_family.clone())
                                       .child(change.name.clone()))
                           .children(change.why.clone().map(|why| {
                                                           div().text_xs()
                                                                .text_color(cx.theme()
                                                                              .muted_foreground)
                                                                .overflow_hidden()
                                                                .whitespace_nowrap()
                                                                .text_ellipsis()
                                                                .child(why)
                                                       })))
            .child(row_actions_button(SharedString::from(format!("openspec-actions-{}",
                                                                 change.dir.display())),
                                      move |popup, window, cx| menu.fill(popup, window, cx)))
            .context_menu(move |popup, window, cx| context.fill(popup, window, cx))
            .into_any_element()
}

/// What a change row's Copy name copies and its Reveal in Finder reveals:
/// the change's name, and its directory in the first working tree it was
/// found in - the one `openspec_changes::merge` keeps.
fn change_actions(change: &ChangeEntry) -> (String, PathBuf) {
    (change.name.clone(), change.dir.clone())
}

/// A change row's menu, the same in its context menu and its actions menu.
#[derive(Clone)]
struct ChangeMenu {
    name:    String,
    /// The change's directory in the first working tree it was found in.
    dir:     PathBuf,
    targets: Vec<SendTarget>,
    entity:  Entity<WorkspaceWindow>,
}

impl ChangeMenu {
    fn fill(&self, menu: PopupMenu, window: &mut Window, cx: &mut Context<PopupMenu>) -> PopupMenu {
        let dir = self.dir.clone();
        let name = self.name.clone();
        let menu = if crate::open_in::can_reveal_files() {
            menu.item(PopupMenuItem::new(knot_core::l10n::t("openspec_changes.reveal"))
                          .on_click(move |_, _, _| {
                              crate::open_in::reveal_path(&dir);
                          }))
        }
        else {
            menu
        };
        let menu = menu.item(PopupMenuItem::new(knot_core::l10n::t("openspec_changes.copy_name"))
                                 .on_click(move |_, _, app: &mut App| {
                                     WorkspaceWindow::copy_to_clipboard(name.clone(), app);
                                 }));
        with_send_prompt(menu,
                         WorkItemRef::Change(self.name.clone()),
                         self.targets.clone(),
                         self.entity.clone(),
                         window,
                         cx)
    }
}

#[cfg(test)]
mod tests;
