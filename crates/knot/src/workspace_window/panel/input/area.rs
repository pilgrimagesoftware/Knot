//! The composer's outer frame: the four rows stacked in order, and the
//! Finder drop target that wraps all of them.
//!
//! Everything this draws is drawn by a sibling. What lives here is the
//! order they appear in and the one behaviour that belongs to the area
//! rather than to any row - a file dropped anywhere in it attaches.

use std::path::PathBuf;

use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use uuid::Uuid;

use crate::composer_style::Palette;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::panel::prompt::PanelInputState;
use crate::workspace_window::prompt_queue::QueuedPanelPrompt;

impl WorkspaceWindow {
    /// The input area: attached-context chips, the expandable text entry,
    /// and a control row (add-context, permission mode, model, effort,
    /// expand/collapse, send) - a sibling of the message list under
    /// `render_panel_pane`, per design decision "Control bar placement".
    #[allow(clippy::too_many_arguments)]
    pub(in crate::workspace_window) fn render_panel_input_area(&mut self, id: Uuid,
                                                               input: &Entity<PanelInputState>,
                                                               pending_context: &[PathBuf],
                                                               queued_prompts: &[QueuedPanelPrompt],
                                                               expanded: bool, blocked: bool,
                                                               turn_active: bool,
                                                               config_options: &[knot_acp::ConfigOption],
                                                               cx: &mut Context<Self>)
                                                               -> impl IntoElement + use<> {
        if !turn_active {
            self.panel_stopping.remove(&id);
        }
        // An appearance switch re-renders without editing, so this frame
        // is the only place a theme change can reach the styling. It
        // compares a palette and returns unless the appearance actually
        // flipped - spans do not depend on the theme, so this repaints and
        // never rescans.
        self.repaint_panel_styling_for_theme(id, Palette::of(cx), cx);
        // Built before the element chain below, which borrows `self`
        // immutably: the lookup's registry is memoized per agent, so
        // producing the popup needs `&mut self`.
        let lookup = self.render_panel_lookup(id, input, cx);
        v_flex().flex_shrink_0()
                .gap_2()
                .p_2()
                .border_t_1()
                .border_color(cx.theme().border)
                .children(Self::render_panel_queued_prompts(id, queued_prompts, cx))
                // Files and images dragged from Finder attach the same way the
                // paperclip and a pasted screenshot do, per `acp-panel-ui`'s
                // attached-context requirement.
                .drag_over::<gpui_kit::ExternalPaths>(|style, _, _, app| {
                    style.bg(app.theme().accent)
                })
                .on_drop(cx.listener(move |view, paths: &gpui_kit::ExternalPaths, _, cx| {
                               for path in paths.paths() {
                                   view.panel_pending_context
                                       .entry(id)
                                       .or_default()
                                       .push(path.clone());
                                   view.queue_attachment_reference(id, path.clone());
                               }
                               view.restyle_panel_attachments(id, Palette::of(cx), cx);
                               cx.notify();
                           }))
                .children(Self::render_panel_context_chips(id, pending_context, cx))
                // The slash lookup sits directly above the prompt row: inside
                // the input area, so the conversation's scroll container cannot
                // clip it, and above the caret rather than over the line being
                // typed.
                .children(lookup)
                .child(self.render_panel_entry_row(id, input, blocked, turn_active, cx))
                .child(self.render_panel_control_bar(id, expanded, config_options, cx))
    }
}
