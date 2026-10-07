//! The Changes view's OpenSpec tab (#504).

use gpui_kit::{Context, IntoElement, Window};

use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// The OpenSpec tab's body.
    pub(super) fn openspec_body(&mut self, _window: &mut Window, _cx: &mut Context<Self>)
                                -> gpui_kit::AnyElement {
        gpui_kit::div().into_any_element()
    }
}
