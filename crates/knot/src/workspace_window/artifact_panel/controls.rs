//! The controls that act on what an artifact section shows, and the one that
//! brings a closed panel back (`openspec/specs/artifact-panel`, "The markdown
//! section can be approved or reviewed", "Each section offers actions on its
//! artifact" and "A closed panel can be reopened").
//!
//! Drawn from memory only - the document cache, the closed-artifact record and
//! the settings - because every one of them is on the render path.

use std::path::Path;
use std::path::PathBuf;

use gpui_kit::ClipboardItem;
use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::assets::IconName;
use gpui_kit::base::Disableable;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use uuid::Uuid;

use super::review::ReviewState;
use super::review::stepped_font_size;
use crate::workspace_window::WorkspaceWindow;

/// A small ghost icon button with a localized tooltip, the shape every
/// header control but the review verdicts takes.
fn icon_button(id: (&'static str, u64), icon: IconName, tooltip: &str) -> Button {
    Button::new(id).icon(icon)
                   .ghost()
                   .small()
                   .tooltip(knot_core::l10n::t(tooltip))
}

/// A section's element id for `id`: the agent's, folded to what `ElementId`
/// takes, as every per-agent control in the panel does.
fn element(name: &'static str, id: Uuid) -> (&'static str, u64) {
    (name, id.as_u128() as u64)
}

impl WorkspaceWindow {
    /// The markdown section's header actions: the review verdicts, the font
    /// size steps, copy, and reveal in Finder.
    pub(in crate::workspace_window) fn markdown_section_actions(&self, id: Uuid, file: &Path,
                                                                cx: &mut Context<Self>)
                                                                -> Vec<gpui_kit::AnyElement> {
        let mut actions = self.review_actions(id, file, cx);

        let size = crate::settings_global::read(cx).markdown_font_size;
        for (name, icon, tooltip, step) in
            [("markdown-font-smaller", IconName::AArrowDown, "artifact_panel.font_smaller", -1),
             ("markdown-font-larger", IconName::AArrowUp, "artifact_panel.font_larger", 1)]
        {
            actions.push(icon_button(element(name, id), icon, tooltip)
                .disabled(stepped_font_size(size, step).is_none())
                .on_click(cx.listener(move |view, _, _window, cx| {
                    view.step_markdown_font_size(step, cx);
                }))
                .into_any_element());
        }

        // Copies what the section shows, not the path: the path is in the
        // header already, and the contents are what a user copying from a
        // rendered file is after. Disabled until the first read lands.
        let body = self.markdown_document_body(id);
        actions.push(icon_button(element("markdown-copy", id),
                                 IconName::Copy,
                                 "artifact_panel.copy_markdown")
            .disabled(body.is_none())
            .on_click(move |_, _window, cx| {
                if let Some(body) = &body {
                    cx.write_to_clipboard(ClipboardItem::new_string(body.to_string()));
                }
            })
            .into_any_element());

        if crate::open_in::can_reveal_files() {
            let file: PathBuf = file.to_path_buf();
            actions.push(icon_button(element("markdown-reveal", id),
                                     IconName::FolderOpen,
                                     "artifact_panel.reveal_in_finder")
                .on_click(move |_, _window, _cx| {
                    // Quietly, like the "Open In" menu: there is nothing the
                    // user can do from here about Finder refusing.
                    crate::open_in::reveal_path(&file);
                })
                .into_any_element());
        }
        actions
    }

    /// Approve and Review while the user is reading, Submit Review once a
    /// review has started, and nothing once a verdict is sent.
    fn review_actions(&self, id: Uuid, file: &Path, cx: &mut Context<Self>)
                      -> Vec<gpui_kit::AnyElement> {
        let file_name = file.file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| file.to_string_lossy().into_owned());
        match self.markdown_review(id) {
            ReviewState::Viewing => {
                vec![Button::new(element("markdown-approve", id))
                         .icon(IconName::CircleCheck)
                         .label(knot_core::l10n::t("artifact_panel.approve"))
                         .success()
                         .small()
                         .tooltip(knot_core::l10n::t("artifact_panel.approve_tooltip"))
                         .on_click(cx.listener(move |view, _, _window, cx| {
                             view.approve_markdown(id, cx);
                         }))
                         .into_any_element(),
                     Button::new(element("markdown-review", id))
                         .icon(IconName::Search)
                         .label(knot_core::l10n::t("artifact_panel.review"))
                         .info()
                         .small()
                         .tooltip(knot_core::l10n::t("artifact_panel.review_tooltip"))
                         .on_click(cx.listener(move |view, _, window, cx| {
                             view.start_markdown_review(id, &file_name, window, cx);
                         }))
                         .into_any_element(),]
            }
            ReviewState::Reviewing => {
                vec![Button::new(element("markdown-submit-review", id))
                         .icon(IconName::Send)
                         .label(knot_core::l10n::t("artifact_panel.submit_review"))
                         .success()
                         .small()
                         .tooltip(knot_core::l10n::t("artifact_panel.submit_review_tooltip"))
                         .on_click(cx.listener(move |view, _, window, cx| {
                             view.submit_markdown_review(id, window, cx);
                         }))
                         .into_any_element(),]
            }
            ReviewState::Submitted => Vec::new(),
        }
    }

    /// The diagram section's header actions: copy its source.
    ///
    /// The source rather than an image: the port draws the diagram as cards,
    /// not a picture, and the source is what an agent or a document can take.
    pub(in crate::workspace_window) fn mermaid_section_actions(&self, id: Uuid, source: &str)
                                                               -> Vec<gpui_kit::AnyElement> {
        let source = source.to_string();
        vec![icon_button(element("mermaid-copy", id),
                         IconName::Copy,
                         "artifact_panel.copy_diagram").on_click(move |_, _window, cx| {
                 cx.write_to_clipboard(ClipboardItem::new_string(source.clone()));
             })
             .into_any_element()]
    }

    /// The header control that reopens `id`'s closed artifacts, or `None`
    /// when there is nothing closed to bring back.
    pub(in crate::workspace_window) fn reopen_artifacts_button(&self, id: Uuid,
                                                               cx: &mut Context<Self>)
                                                               -> Option<gpui_kit::AnyElement> {
        if self.reopenable_artifacts(id).is_empty() {
            return None;
        }
        Some(icon_button(element("artifact-panel-reopen", id),
                         IconName::PanelRightOpen,
                         "artifact_panel.reopen")
            .on_click(cx.listener(move |view, _, _window, cx| {
                view.reopen_artifacts(id);
                cx.notify();
            }))
            .into_any_element())
    }
}
