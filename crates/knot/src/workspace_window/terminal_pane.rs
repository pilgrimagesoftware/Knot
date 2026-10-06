//! A shell agent's terminal surface: the `gpui_terminal::TerminalView` that
//! draws its session, and what the window does with the view's events.
//!
//! The view owns input, sizing and repaint; the window owns the agent store
//! and the pasteboard policy, so the two events that need either - a title, an
//! OSC 52 clipboard write - come back here.

use gpui_kit::{
    App, AppContext as _, ClipboardItem, Context, Entity, Focusable as _, Subscription, px,
};
use gpui_terminal::{PtyTransport, Terminal, TerminalEvent, TerminalStyle, TerminalView};
use uuid::Uuid;

use crate::workspace_window::WorkspaceWindow;

/// One agent's view, and the subscription that routes its events.
///
/// Created with the session rather than on first draw: titles and clipboard
/// writes from an agent nobody is looking at still have to land, and the
/// view's pump is what delivers them.
pub(super) struct TerminalPane {
    pub(super) view: Entity<TerminalView<PtyTransport>>,
    _events:         Subscription,
}

impl WorkspaceWindow {
    /// Builds `id`'s view over `terminal` and starts listening to it.
    pub(super) fn open_terminal_pane(&mut self, id: Uuid, terminal: Terminal<PtyTransport>,
                                     cx: &mut Context<Self>) {
        let style = self.terminal_style(cx);
        let view = cx.new(|cx| TerminalView::new(terminal, style, cx));
        let events = cx.subscribe(&view, move |window, _, event, cx| {
                           window.on_terminal_event(id, event, cx);
                       });
        self.terminal_panes.insert(id,
                                   TerminalPane { view,
                                                  _events: events });
    }

    fn on_terminal_event(&mut self, id: Uuid, event: &TerminalEvent, cx: &mut Context<Self>) {
        match event {
            TerminalEvent::Title(title) => {
                self.store.lock().set_terminal_title(id, title.clone());
                cx.notify();
            }
            TerminalEvent::ClipboardStore(text) => {
                cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
            }
            TerminalEvent::TransportFailed(error) => {
                eprintln!("agent {id}'s terminal failed: {error}");
            }
            // Exit is already handled on the reader thread by
            // `ensure_session`'s hook, which queues the agent for removal; a
            // reset title and the bell have no surface in Knot yet.
            _ => {}
        }
    }

    /// Brings the selected agent's view up to the configured font. A no-op
    /// on the frames where nothing changed - `set_style` compares first - so
    /// it runs every frame, after `refresh_terminal_font`.
    pub(super) fn sync_terminal_style(&mut self, cx: &mut Context<Self>) {
        let Some(pane) = self.selected_agent
                             .and_then(|id| self.terminal_panes.get(&id))
        else {
            return;
        };
        let style = self.terminal_style(cx);
        pane.view
            .clone()
            .update(cx, |view, cx| view.set_style(style, cx));
    }

    /// Focuses `id`'s terminal surface, if it has one.
    pub(super) fn focus_terminal(&self, id: Uuid, window: &mut gpui_kit::Window, cx: &mut App) {
        if let Some(pane) = self.terminal_panes.get(&id) {
            window.focus(&pane.view.focus_handle(cx), cx);
        }
    }

    fn terminal_style(&self, cx: &App) -> TerminalStyle {
        TerminalStyle::new(self.terminal_font_family(),
                           px(crate::settings_global::read(cx).terminal_font_size as f32))
    }
}
