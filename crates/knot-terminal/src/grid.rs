//! PTY-output-to-grid parsing, backed by `alacritty_terminal`. Feeds raw PTY
//! bytes into a VT/ANSI parser and exposes the resulting cell grid plus
//! terminal-originated events (title changes, clipboard requests, bell) -
//! see `openspec/changes/terminal-rendering/design.md`.

use std::sync::mpsc;

pub use alacritty_terminal::event::Event as GridEvent;
use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
pub use alacritty_terminal::term::ClipboardType;
pub use alacritty_terminal::term::cell::Cell;
use alacritty_terminal::term::{Config as TermConfig, Term};
use alacritty_terminal::vte::ansi::Processor;

/// A terminal's fixed size, in columns and (visible) rows. `alacritty_terminal`
/// also tracks scrollback beyond `rows`, addressed separately from the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSize {
    pub columns: usize,
    pub rows:    usize,
}

impl Dimensions for GridSize {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.columns
    }
}

/// Forwards `alacritty_terminal` events to an [`mpsc::Sender`] so [`Grid`]
/// can drain them after each `feed` - `EventListener::send_event` takes
/// `&self`, so a channel (not a plain `Vec`) is the simplest way to record
/// events without a lock.
#[derive(Clone)]
struct EventForwarder(mpsc::Sender<Event>);

impl EventListener for EventForwarder {
    fn send_event(&self, event: Event) {
        let _ = self.0.send(event);
    }
}

/// One agent's parsed terminal state: feed it raw PTY output, read back the
/// visible cell grid and any events (title changes, clipboard requests,
/// bell) the running program triggered.
pub struct Grid {
    term:   Term<EventForwarder>,
    parser: Processor,
    events: mpsc::Receiver<Event>,
    /// Set whenever the visible grid changes (feed/resize/selection); a UI
    /// poll loop reads and clears this via [`Self::take_dirty`] to decide
    /// whether a repaint is actually needed, since the PTY reader thread
    /// that calls `feed` has no way to trigger one itself.
    dirty:  bool,
}

impl Grid {
    pub fn new(size: GridSize) -> Self {
        let (tx, rx) = mpsc::channel();
        let term = Term::new(TermConfig::default(), &size, EventForwarder(tx));
        Self { term,
               parser: Processor::new(),
               events: rx,
               dirty: false }
    }

    /// Parses `bytes` (raw PTY output) into the grid, updating cell
    /// contents, cursor position, and queuing any resulting events.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.term, bytes);
        self.dirty = true;
    }

    /// Resizes the grid's row/column count. Does not resize the PTY itself -
    /// callers own that separately (see `TerminalTransport`).
    pub fn resize(&mut self, size: GridSize) {
        self.term.resize(size);
        self.dirty = true;
    }

    /// Returns whether the grid has changed since the last call, clearing
    /// the flag.
    pub fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    /// Drains and returns every event queued since the last call.
    pub fn drain_events(&mut self) -> Vec<GridEvent> {
        self.events.try_iter().collect()
    }

    /// The cursor's current position (0-indexed column/line within the
    /// visible grid).
    pub fn cursor(&self) -> (usize, usize) {
        let point = self.term.grid().cursor.point;
        (point.column.0, point.line.0.max(0) as usize)
    }

    /// Whether the running program has enabled SGR mouse reporting - if
    /// not, mouse events should drive the grid's own scrollback/selection
    /// instead of being sent to the PTY (see `terminal-input`'s spec).
    pub fn sgr_mouse_mode(&self) -> bool {
        use alacritty_terminal::term::TermMode;
        let mode = self.term.mode();
        mode.contains(TermMode::SGR_MOUSE) && mode.intersects(TermMode::MOUSE_MODE)
    }

    /// Starts (replacing any existing) a simple text selection anchored at
    /// this cell.
    pub fn start_selection(&mut self, column: usize, row: usize) {
        let point = Point::new(Line(row as i32), Column(column));
        self.term.selection = Some(Selection::new(SelectionType::Simple, point, Side::Left));
        self.dirty = true;
    }

    /// Extends the in-progress selection (if any) to this cell.
    pub fn update_selection(&mut self, column: usize, row: usize) {
        if let Some(selection) = &mut self.term.selection {
            let point = Point::new(Line(row as i32), Column(column));
            selection.update(point, Side::Right);
            self.dirty = true;
        }
    }

    /// Clears any active selection.
    pub fn clear_selection(&mut self) {
        self.term.selection = None;
    }

    /// The selected text, if any (empty/point-only selections return
    /// `None`).
    pub fn selection_text(&self) -> Option<String> {
        self.term.selection_to_string()
    }

    /// Whether this cell is part of the active selection - for rendering a
    /// highlight.
    pub fn is_selected(&self, column: usize, row: usize) -> bool {
        let point = Point::new(Line(row as i32), Column(column));
        self.term
            .selection
            .as_ref()
            .and_then(|selection| selection.to_range(&self.term))
            .is_some_and(|range| range.contains(point))
    }

    /// A visible row's cells, left to right. Panics if `row` is out of
    /// bounds for the grid's current size.
    pub fn row_cells(&self, row: usize) -> Vec<Cell> {
        self.term.grid()[Line(row as i32)].into_iter()
                                          .cloned()
                                          .collect()
    }

    /// A visible row's text content, with trailing blank cells trimmed -
    /// convenient for tests and any plain-text consumer.
    pub fn row_text(&self, row: usize) -> String {
        let mut text: String = self.row_cells(row).iter().map(|cell| cell.c).collect();
        while text.ends_with(' ') {
            text.pop();
        }
        text
    }

    pub fn size(&self) -> GridSize {
        GridSize { columns: self.term.columns(),
                   rows:    self.term.screen_lines(), }
    }
}

#[allow(dead_code)]
fn point_at(column: usize, row: usize) -> Point {
    Point::new(Line(row as i32), Column(column))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(columns: usize, rows: usize) -> Grid {
        Grid::new(GridSize { columns, rows })
    }

    #[test]
    fn feeds_plain_text_into_the_grid() {
        let mut grid = grid(20, 5);
        grid.feed(b"hello");
        assert_eq!(grid.row_text(0), "hello");
    }

    #[test]
    fn alternate_screen_content_and_cursor_are_reflected() {
        let mut grid = grid(20, 5);
        grid.feed(b"primary screen text");
        // Enter the alternate screen buffer (what full-screen TUIs like
        // Claude Code's use), clear it, and draw new content plus move the
        // cursor - both should read from the now-active alt screen, not the
        // primary one still holding "primary screen text".
        grid.feed(b"\x1b[?1049h\x1b[2J\x1b[H");
        grid.feed(b"alt screen line");
        grid.feed(b"\x1b[3;1Hprompt row");
        assert_eq!(grid.row_text(0), "alt screen line");
        assert_eq!(grid.row_text(2), "prompt row");
        assert_eq!(grid.cursor(), (10, 2));

        // Leaving the alt screen restores the primary content untouched.
        grid.feed(b"\x1b[?1049l");
        assert_eq!(grid.row_text(0), "primary screen text");
    }

    #[test]
    fn tracks_cursor_position_after_writes() {
        let mut grid = grid(20, 5);
        grid.feed(b"hi");
        assert_eq!(grid.cursor(), (2, 0));
    }

    #[test]
    fn newline_and_carriage_return_move_to_the_next_line() {
        let mut grid = grid(20, 5);
        grid.feed(b"one\r\ntwo");
        assert_eq!(grid.row_text(0), "one");
        assert_eq!(grid.row_text(1), "two");
    }

    #[test]
    fn resize_updates_reported_size() {
        let mut grid = grid(20, 5);
        grid.resize(GridSize { columns: 40,
                               rows:    10, });
        assert_eq!(grid.size(),
                   GridSize { columns: 40,
                              rows:    10, });
    }

    #[test]
    fn title_escape_sequence_produces_a_title_event() {
        let mut grid = grid(20, 5);
        grid.feed(b"\x1b]0;my title\x07");
        let events = grid.drain_events();
        assert!(events.iter().any(|event| matches!(
                                 event,
                                 GridEvent::Title(title) if title == "my title"
                             )));
    }

    #[test]
    fn sgr_color_codes_set_cell_foreground() {
        use alacritty_terminal::term::cell::Flags;

        let mut grid = grid(20, 5);
        grid.feed(b"\x1b[1mbold");
        let cells = grid.row_cells(0);
        assert!(cells[0].flags.contains(Flags::BOLD));
    }

    #[test]
    fn no_selection_text_before_selecting() {
        let mut grid = grid(20, 5);
        grid.feed(b"hello world");
        assert_eq!(grid.selection_text(), None);
    }

    #[test]
    fn dragging_a_selection_captures_the_spanned_text() {
        let mut grid = grid(20, 5);
        grid.feed(b"hello world");
        grid.start_selection(0, 0);
        grid.update_selection(4, 0);
        assert_eq!(grid.selection_text(), Some("hello".to_string()));
    }

    #[test]
    fn is_selected_reports_cells_within_the_selection() {
        let mut grid = grid(20, 5);
        grid.feed(b"hello world");
        grid.start_selection(0, 0);
        grid.update_selection(4, 0);
        assert!(grid.is_selected(0, 0));
        assert!(grid.is_selected(4, 0));
        assert!(!grid.is_selected(6, 0));
    }

    #[test]
    fn clear_selection_removes_it() {
        let mut grid = grid(20, 5);
        grid.feed(b"hello world");
        grid.start_selection(0, 0);
        grid.update_selection(4, 0);
        grid.clear_selection();
        assert_eq!(grid.selection_text(), None);
        assert!(!grid.is_selected(0, 0));
    }

    #[test]
    fn take_dirty_reports_and_clears_changes() {
        let mut grid = grid(20, 5);
        assert!(!grid.take_dirty(), "a fresh grid has nothing to repaint");

        grid.feed(b"hi");
        assert!(grid.take_dirty());
        assert!(!grid.take_dirty(), "dirty flag clears after being read");

        grid.resize(GridSize { columns: 30,
                               rows:    10, });
        assert!(grid.take_dirty());
    }
}
