//! Input routing for the terminal pane: key presses, mouse buttons, drag
//! selection, scroll, copy, and keeping the PTY's grid sized to the pane.
//!
//! Each of these translates a GPUI event into the `knot-terminal` session's
//! vocabulary and does nothing else; the session owns all the state.

use gpui_kit::App;
use gpui_kit::ClipboardItem;
use gpui_kit::Window;
use gpui_kit::px;
use uuid::Uuid;

use crate::workspace_window::TERMINAL_HEADER_HEIGHT;
use crate::workspace_window::TERMINAL_SIDEBAR_WIDTH;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::terminal_cell_size;
use crate::workspace_window::terminal_font_family;

impl WorkspaceWindow {
    /// Resizes `id`'s session grid/PTY to match the content pane's current
    /// size, if it changed.
    pub(super) fn resize_session_to_pane(&mut self, id: Uuid, window: &Window, cx: &App) {
        let Some(session) = self.sessions.get(&id)
        else {
            return;
        };
        let (cell_width, cell_height) =
            terminal_cell_size(cx,
                               terminal_font_family(&self.settings, cx),
                               px(self.settings.terminal_font_size as f32));
        let viewport = window.viewport_size();
        let pane_width =
            (f32::from(viewport.width) - self.sidebar_width(cx) as f32).max(cell_width);
        let pane_height = (f32::from(viewport.height) - TERMINAL_HEADER_HEIGHT).max(cell_height);
        let size = knot_terminal::GridSize { columns: (pane_width / cell_width) as usize,
                                             rows:    (pane_height / cell_height) as usize, };

        let current = session.lock().grid().map(|grid| grid.lock().size());
        if current != Some(size) {
            let mut session = session.lock();
            let _ = session.resize(size);
        }
    }

    /// Translates a key press on the focused terminal pane into PTY input,
    /// per `terminal-input`'s spec.
    pub(super) fn dispatch_key(&mut self, id: Uuid, event: &gpui_kit::KeyDownEvent, cx: &mut App) {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform && keystroke.key == "c" {
            self.copy_selection(id, cx);
            return;
        }
        if keystroke.modifiers.platform {
            return;
        }
        let Some(session) = self.sessions.get(&id)
        else {
            return;
        };
        let input = knot_terminal::KeyInput { key:      &keystroke.key,
                                              key_char: keystroke.key_char.as_deref(),
                                              control:  keystroke.modifiers.control,
                                              alt:      keystroke.modifiers.alt, };
        let Some(bytes) = knot_terminal::key_to_bytes(input)
        else {
            return;
        };
        let Ok(text) = String::from_utf8(bytes)
        else {
            return;
        };
        {
            let mut session = session.lock();
            let _ = session.send_text(&text);
        }
    }

    /// Converts a window-relative pixel position to a 0-indexed grid
    /// column/row, using the same pane geometry as `resize_session_to_pane`.
    pub(super) fn grid_position(&self, position: gpui_kit::Point<gpui_kit::Pixels>, cx: &App)
                                -> (usize, usize) {
        let (cell_width, cell_height) =
            terminal_cell_size(cx,
                               terminal_font_family(&self.settings, cx),
                               px(self.settings.terminal_font_size as f32));
        let x = (f32::from(position.x) - self.sidebar_width(cx) as f32).max(0.);
        let y = (f32::from(position.y) - TERMINAL_HEADER_HEIGHT).max(0.);
        ((x / cell_width) as usize, (y / cell_height) as usize)
    }

    /// Sends a mouse button press/release to the focused terminal pane's
    /// session, if the running program has enabled SGR mouse reporting -
    /// otherwise a no-op (falls back to no interaction rather than a
    /// scrollback/selection view, which isn't implemented yet).
    pub(super) fn dispatch_mouse_button(&mut self, id: Uuid,
                                        position: gpui_kit::Point<gpui_kit::Pixels>,
                                        button: knot_terminal::MouseButton, pressed: bool,
                                        cx: &App) {
        let Some(session) = self.sessions.get(&id)
        else {
            return;
        };
        let Some(grid) = session.lock().grid()
        else {
            return;
        };
        let (column, row) = self.grid_position(position, cx);
        let sgr = grid.lock().sgr_mouse_mode();
        if !sgr {
            // No mouse-aware program is listening - left-button press starts
            // (replacing any prior) text selection instead of forwarding the
            // click, per terminal-input's spec.
            if button == knot_terminal::MouseButton::Left && pressed {
                grid.lock().start_selection(column, row);
            }
            return;
        }
        let Some(bytes) = knot_terminal::mouse_to_bytes(knot_terminal::MouseInput { row,
                                                                                    column,
                                                                                    button,
                                                                                    pressed },
                                                        sgr)
        else {
            return;
        };
        if let Ok(text) = String::from_utf8(bytes) {
            let _ = session.lock().send_text(&text);
        }
    }

    /// Extends an in-progress text selection while the mouse is dragged
    /// with the left button held, when no mouse-aware program has claimed
    /// mouse reporting.
    pub(super) fn dispatch_mouse_drag(&mut self, id: Uuid,
                                      position: gpui_kit::Point<gpui_kit::Pixels>, cx: &App) {
        let Some(session) = self.sessions.get(&id)
        else {
            return;
        };
        let Some(grid) = session.lock().grid()
        else {
            return;
        };
        let mut grid = grid.lock();
        if grid.sgr_mouse_mode() {
            return;
        }
        let (column, row) = self.grid_position(position, cx);
        grid.update_selection(column, row);
    }

    /// Copies the focused terminal pane's active selection to the OS
    /// pasteboard, applying the terminal-actions spec's default transform
    /// (trim trailing whitespace per line).
    pub(super) fn copy_selection(&mut self, id: Uuid, cx: &mut App) {
        let Some(session) = self.sessions.get(&id)
        else {
            return;
        };
        let Some(text) = session.lock()
                                .grid()
                                .and_then(|grid| grid.lock().selection_text())
        else {
            return;
        };
        let text: String = text.lines()
                               .map(str::trim_end)
                               .collect::<Vec<_>>()
                               .join("\n");
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }

    /// Sends a scroll-wheel event to the focused terminal pane's session
    /// when the running program has enabled SGR mouse reporting.
    pub(super) fn dispatch_scroll(&mut self, id: Uuid,
                                  position: gpui_kit::Point<gpui_kit::Pixels>, lines: f32,
                                  cx: &App) {
        if lines == 0. {
            return;
        }
        let button = if lines > 0. {
            knot_terminal::MouseButton::WheelUp
        }
        else {
            knot_terminal::MouseButton::WheelDown
        };
        self.dispatch_mouse_button(id, position, button, true, cx);
    }
}
