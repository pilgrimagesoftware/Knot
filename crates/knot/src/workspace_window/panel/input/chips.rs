//! The strip of attached-context chips between the queued prompts and the
//! composer: one chip per pending path, each with its own dismissal.
//!
//! The strip is only the presentation. What is attached lives in
//! `panel_pending_context`, and removing a chip goes through
//! [`WorkspaceWindow::remove_panel_context`] rather than editing the strip.

use std::path::PathBuf;

use gpui_kit::ClickEvent;
use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::assets::IconName;
use gpui_kit::base::h_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::div;
use uuid::Uuid;

use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// The attached-context chips for `id`, or nothing when none are
    /// pending.
    pub(super) fn render_panel_context_chips(id: Uuid, pending_context: &[PathBuf],
                                             cx: &mut Context<Self>)
                                             -> Option<impl IntoElement + use<>> {
        (!pending_context.is_empty()).then(|| {
                                         h_flex().gap_1()
                                                 .flex_wrap()
                                                 .children(pending_context.iter().enumerate().map(
                |(index, path)| {
                    let file_name = path.file_name()
                                        .map(|name| name.to_string_lossy().into_owned())
                                        .unwrap_or_else(|| path.to_string_lossy().into_owned());
                    h_flex()
                            .gap_1()
                            .items_center()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .bg(cx.theme().secondary)
                            .child(div().text_xs().child(file_name))
                            .child(
                                Button::new(("panel-remove-context", index as u64))
                                    .icon(IconName::CircleX)
                                    .ghost()
                                    .xsmall()
                                    .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                                        view.remove_panel_context(id, index);
                                        cx.notify();
                                    })),
                            )
                },
            ))
                                     })
    }
}
