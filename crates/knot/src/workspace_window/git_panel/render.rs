//! The panel's element tree: the header, the branch line, the grouped working
//! tree, and the diff pane beneath it.
//!
//! Nothing here runs git. The frame draws what
//! [`WorkspaceWindow::git_panel_view`] copied out of the caches and asks for
//! anything that has aged out on the way; the answer arrives in a later
//! frame. That rule is why `crate::refresh_cache` exists.
//!
//! Contract: `openspec/specs/git-panel-ui/spec.md`.

use gpui_kit::assets::IconName;
use gpui_kit::base::{StyledExt, h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{ActiveTheme, Icon, Sizable};
use gpui_kit::{
    ClickEvent, Context, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window, div, px,
};

use super::actions::GitAction;
use super::view::{PanelContent, PanelView, TreeView, change_glyph};
use crate::git_panel::sections::{Row, Section, SectionKind, paths_of};
use crate::workspace_window::WorkspaceWindow;

/// A stable element id per row: the section and the path, so the two rows of
/// a staged-and-modified path do not collide.
fn row_id(row: &Row) -> SharedString {
    format!("git-row-{:?}-{}", row.section, row.path.display()).into()
}

impl WorkspaceWindow {
    /// The git panel for `id`, or `None` when it is not open.
    pub(in crate::workspace_window) fn git_panel_pane(&mut self, id: uuid::Uuid, folder: &str,
                                                      window: &mut Window,
                                                      cx: &mut Context<Self>)
                                                      -> Option<gpui_kit::AnyElement> {
        if !self.is_git_panel_open(id) {
            return None;
        }

        let view = self.git_panel_view(id, folder);
        Some(self.render_git_panel(view, window, cx))
    }

    fn render_git_panel(&mut self, view: PanelView, window: &mut Window, cx: &mut Context<Self>)
                        -> gpui_kit::AnyElement {
        let id = view.agent;
        let has_staged = matches!(&view.content, PanelContent::Changes(tree) if tree.staged);

        v_flex().h_full()
                .w(px(view.width))
                .flex_none()
                .border_l_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().background)
                .child(self.git_panel_header(id, &view.folder, has_staged, cx))
                .children(view.error.clone().map(|text| {
                                                div().px_4()
                                                     .py_2()
                                                     .text_sm()
                                                     .text_color(cx.theme().danger)
                                                     .child(text)
                                            }))
                .child(self.git_panel_body(view, window, cx))
                .into_any_element()
    }

    /// Title, the commit control, refresh and close.
    ///
    /// The commit control is present only while something is staged: with an
    /// empty index `git commit` can only fail, so offering it is offering a
    /// button whose single outcome is an error.
    fn git_panel_header(&self, id: uuid::Uuid, folder: &str, has_staged: bool,
                        cx: &mut Context<Self>)
                        -> gpui_kit::AnyElement {
        let commit_folder = folder.to_string();

        h_flex().w_full()
                .flex_shrink_0()
                .items_center()
                .justify_between()
                .gap_2()
                .px_4()
                .py_3()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(div().text_sm()
                            .font_semibold()
                            .child(knot_core::l10n::t("git_panel.title")))
                .child(h_flex()
                    .flex_shrink_0()
                    .gap_1()
                    .children(has_staged.then(|| {
                        Button::new("git-panel-commit")
                            .icon(IconName::Check)
                            .label(knot_core::l10n::t("git_panel.commit"))
                            .primary()
                            .on_click(cx.listener(move |view, _: &ClickEvent, window, cx| {
                                view.open_commit_window(id, &commit_folder, window, cx);
                            }))
                    }))
                    .child(Button::new("git-panel-refresh")
                        .icon(IconName::RefreshCw)
                        .ghost()
                        .tooltip(knot_core::l10n::t("git_panel.refresh"))
                        .on_click(cx.listener(move |view, _: &ClickEvent, _window, cx| {
                            view.refresh_git_panel(id, cx);
                        })))
                    .child(Button::new("git-panel-close")
                        .icon(IconName::Close)
                        .ghost()
                        .tooltip(knot_core::l10n::t("git_panel.close"))
                        .on_click(cx.listener(move |view, _: &ClickEvent, _window, cx| {
                            view.close_git_panel(id, cx);
                        }))))
                .into_any_element()
    }

    /// The one content state this frame is in.
    fn git_panel_body(&mut self, view: PanelView, window: &mut Window, cx: &mut Context<Self>)
                      -> gpui_kit::AnyElement {
        let PanelView { agent,
                        folder,
                        content,
                        selection,
                        diff,
                        .. } = view;

        match content {
            PanelContent::Loading => centered(knot_core::l10n::t("git_panel.loading"), cx),
            PanelContent::NotARepository => {
                centered(knot_core::l10n::t("git_panel.not_a_repository"), cx)
            }
            PanelContent::Failed(reason) => {
                let text =
                    knot_core::l10n::t_with("git_panel.status_failed", &[("reason", &reason)]);
                centered_danger(text, cx)
            }
            PanelContent::Clean => centered(knot_core::l10n::t("git_panel.clean"), cx),
            PanelContent::Changes(tree) => {
                v_flex().flex_1()
                        .min_h_0()
                        .child(div().id("git-panel-tree")
                                    .flex_1()
                                    .min_h_0()
                                    .overflow_y_scroll()
                                    .child(self.tree_column(agent, &folder, *tree, &selection, cx)))
                        .child(self.git_diff_pane(agent, diff, window, cx))
                        .into_any_element()
            }
        }
    }

    fn tree_column(&self, id: uuid::Uuid, folder: &str, tree: TreeView,
                   selection: &Option<crate::git_panel::state::Selection>,
                   cx: &mut Context<Self>)
                   -> gpui_kit::AnyElement {
        v_flex().w_full()
                .gap_3()
                .p_3()
                .child(branch_line(&tree, cx))
                .children(tree.sections
                              .into_iter()
                              .map(|section| {
                                  self.render_section(id, folder, section, selection, cx)
                              }))
                .into_any_element()
    }

    /// One section: its title, its count, its bulk control, and its rows.
    ///
    /// Only the staged and unstaged sections get a bulk control. Untracked
    /// has none because stage-all (`git add -A`) already covers it, and
    /// conflicts has none because there is no bulk resolution to offer.
    fn render_section(&self, id: uuid::Uuid, folder: &str, section: Section,
                      selection: &Option<crate::git_panel::state::Selection>,
                      cx: &mut Context<Self>)
                      -> gpui_kit::AnyElement {
        let count = section.rows.len();
        let bulk = bulk_action(section.kind);
        let bulk_paths = bulk.and_then(|_| {
                                 paths_of(&section.rows).map(|paths| {
                                                            paths.into_iter()
                                                                 .map(std::path::PathBuf::from)
                                                                 .collect()
                                                        })
                             });
        let folder_for_bulk = folder.to_string();

        v_flex()
            .w_full()
            .gap_1()
            .child(h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .child(h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().text_xs()
                                .font_semibold()
                                .text_color(section_color(section.kind, cx))
                                .child(knot_core::l10n::t(section.kind.title_key())))
                    .child(div().text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(knot_core::l10n::t_with("git_panel.section_count",
                                                               &[("count", &count.to_string())]))))
                .children(bulk.zip(bulk_paths).map(|(action, paths): (GitAction, Vec<_>)| {
                    Button::new(SharedString::from(format!("git-bulk-{:?}", section.kind)))
                        .label(knot_core::l10n::t(bulk_label_key(action)))
                        .ghost()
                        .xsmall()
                        .on_click(cx.listener(move |view, _: &ClickEvent, _window, cx| {
                            view.run_git_action(id, &folder_for_bulk, action, paths.clone(), cx);
                        }))
                })))
            .children(section.rows
                             .into_iter()
                             .map(|row| self.render_row(id, folder, row, selection, cx)))
            .into_any_element()
    }

    /// One row: its change glyph, its name and directory, and the actions
    /// valid for its side.
    fn render_row(&self, id: uuid::Uuid, folder: &str, row: Row,
                  selection: &Option<crate::git_panel::state::Selection>, cx: &mut Context<Self>)
                  -> gpui_kit::AnyElement {
        let row_selection = row.selection();
        let is_selected = selection.as_ref() == Some(&row_selection);
        let name = row.file_name();
        let directory = row.directory();
        let glyph = change_glyph(row.change);
        let color = section_color(row.section, cx);
        let path = row.path.clone();
        let select = row_selection.clone();

        div()
            .id(row_id(&row))
            .cursor_pointer()
            .rounded(cx.theme().radius)
            .px_2()
            .py_1()
            .bg(if is_selected {
                cx.theme().muted
            }
            else {
                cx.theme().transparent
            })
            .on_click(cx.listener(move |view, _: &ClickEvent, _window, cx| {
                view.select_git_row(id, select.clone(), cx);
            }))
            .child(h_flex()
                .w_full()
                .min_w_0()
                .gap_2()
                .items_center()
                .child(div().w(px(16.))
                            .flex_shrink_0()
                            .text_xs()
                            .font_semibold()
                            .text_color(color)
                            .child(glyph))
                // `min_w_0` on the growing child: a long path otherwise
                // pushes the action buttons out past the panel edge.
                .child(v_flex()
                    .flex_1()
                    .min_w_0()
                    .child(div().text_sm().truncate().child(name))
                    .children(directory.map(|dir| {
                        div().text_xs()
                             .text_color(cx.theme().muted_foreground)
                             .truncate()
                             .child(dir)
                    })))
                .child(h_flex().flex_shrink_0()
                               .gap_1()
                               .children(row_actions(row.section).into_iter().map(|action| {
                                             self.row_action_button(id,
                                                                    folder,
                                                                    action,
                                                                    &path,
                                                                    &row,
                                                                    cx)
                                         }))))
            .into_any_element()
    }

    fn row_action_button(&self, id: uuid::Uuid, folder: &str, action: GitAction,
                         path: &std::path::Path, row: &Row, cx: &mut Context<Self>)
                         -> gpui_kit::AnyElement {
        let paths = vec![path.to_path_buf()];
        let folder = folder.to_string();
        let label = knot_core::l10n::t(row_action_label_key(action));
        let display = path.display().to_string();

        Button::new(SharedString::from(format!("{}-{action:?}", row_id(row))))
            .icon(Icon::default().path(row_action_icon(action)))
            .ghost()
            .xsmall()
            .tooltip(label)
            .on_click(cx.listener(move |view, _: &ClickEvent, window, cx| {
                // Discard is the one irreversible action here, so it asks
                // first. Staging and unstaging are each undone by their
                // opposite.
                if action.is_destructive() {
                    view.confirm_discard(id, &folder, paths.clone(), &display, window, cx);
                }
                else {
                    view.run_git_action(id, &folder, action, paths.clone(), cx);
                }
            }))
            .into_any_element()
    }
}

/// The branch, and the ahead/behind counts when there are any.
fn branch_line(tree: &TreeView, cx: &mut Context<WorkspaceWindow>) -> gpui_kit::AnyElement {
    let Some(branch) = tree.branch.clone()
    else {
        return div().text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(knot_core::l10n::t("git_panel.detached_head"))
                    .into_any_element();
    };

    h_flex().w_full()
            .gap_2()
            .items_center()
            .child(Icon::default().path("icons/git-branch.svg"))
            .child(div().text_sm().font_medium().truncate().child(branch))
            .children((tree.ahead > 0).then(|| {
                                          div().text_xs()
                               .text_color(cx.theme().success)
                               .child(knot_core::l10n::t_with("git_panel.ahead",
                                                              &[("count",
                                                                 &tree.ahead.to_string())]))
                                      }))
            .children((tree.behind > 0).then(|| {
                                           div().text_xs()
                               .text_color(cx.theme().warning)
                               .child(knot_core::l10n::t_with("git_panel.behind",
                                                              &[("count",
                                                                 &tree.behind.to_string())]))
                                       }))
            .into_any_element()
}

fn centered(text: String, cx: &mut Context<WorkspaceWindow>) -> gpui_kit::AnyElement {
    v_flex().flex_1()
            .min_h_0()
            .items_center()
            .justify_center()
            .p_4()
            .child(div().text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(text))
            .into_any_element()
}

fn centered_danger(text: String, cx: &mut Context<WorkspaceWindow>) -> gpui_kit::AnyElement {
    v_flex().flex_1()
            .min_h_0()
            .items_center()
            .justify_center()
            .p_4()
            .child(div().text_sm().text_color(cx.theme().danger).child(text))
            .into_any_element()
}

/// Which actions a row on this side offers.
///
/// Discard is absent for an untracked row because `git restore` restores
/// tracked content - on an untracked path it would either do nothing or
/// destroy a file that was never committed. Swift made the same choice.
fn row_actions(section: SectionKind) -> Vec<GitAction> {
    match section {
        SectionKind::Staged => vec![GitAction::Unstage],
        SectionKind::Unstaged => vec![GitAction::Stage, GitAction::Discard],
        SectionKind::Untracked => vec![GitAction::Stage],
        SectionKind::Conflicted => vec![GitAction::Stage],
    }
}

fn bulk_action(section: SectionKind) -> Option<GitAction> {
    match section {
        SectionKind::Staged => Some(GitAction::UnstageAll),
        SectionKind::Unstaged => Some(GitAction::StageAll),
        SectionKind::Untracked | SectionKind::Conflicted => None,
    }
}

fn row_action_label_key(action: GitAction) -> &'static str {
    match action {
        GitAction::Stage => "git_panel.stage",
        GitAction::Unstage => "git_panel.unstage",
        GitAction::Discard => "git_panel.discard",
        GitAction::StageAll => "git_panel.stage_all",
        GitAction::UnstageAll => "git_panel.unstage_all",
    }
}

fn bulk_label_key(action: GitAction) -> &'static str {
    row_action_label_key(action)
}

fn row_action_icon(action: GitAction) -> &'static str {
    match action {
        GitAction::Stage | GitAction::StageAll => "icons/circle-plus.svg",
        GitAction::Unstage | GitAction::UnstageAll => "icons/circle-minus.svg",
        GitAction::Discard => "icons/rotate-ccw.svg",
    }
}

/// The colour a section and its rows are drawn in: staged reads as applied,
/// unstaged as pending, untracked as not yet git's, conflicts as blocking.
fn section_color(section: SectionKind, cx: &mut Context<WorkspaceWindow>) -> gpui_kit::Hsla {
    match section {
        SectionKind::Staged => cx.theme().success,
        SectionKind::Unstaged => cx.theme().warning,
        SectionKind::Untracked => cx.theme().muted_foreground,
        SectionKind::Conflicted => cx.theme().danger,
    }
}
