//! Renders a `knot_terminal::Grid` as GPUI elements - a plain cell-grid
//! renderer (text runs + a cursor), not a native surface. See
//! `openspec/changes/terminal-rendering/design.md`.

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::{IntoElement, ParentElement, Styled, div, rgb};
use knot_terminal::{Cell, Grid};

use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::vte::ansi::{Color, NamedColor, Rgb};

const DEFAULT_FOREGROUND: u32 = 0xE0E0E0;
const DEFAULT_BACKGROUND: u32 = 0x262626;

/// Renders every visible row of `grid` as a monospace text grid, with the
/// cursor cell shown by swapping its foreground/background. `font_family`
/// should come from the user's `terminal_font_name` setting - a name GPUI
/// has actually registered (the default, "JetBrains Mono", is embedded and
/// registered in `apply_visual_identity`), not an arbitrary system font
/// name, which silently falls back to the app's (proportional) UI font if
/// GPUI can't resolve it.
pub(crate) fn render_grid(
    grid: &Grid,
    font_family: gpui_kit::SharedString,
    font_size: gpui_kit::Pixels,
) -> impl IntoElement {
    let size = grid.size();
    let (cursor_col, cursor_row) = grid.cursor();

    v_flex()
        .size_full()
        .font_family(font_family)
        .text_size(font_size)
        .bg(rgb(DEFAULT_BACKGROUND))
        .children(
            (0..size.rows)
                .map(|row| render_row(grid, grid.row_cells(row), row, cursor_row, cursor_col)),
        )
}

fn render_row(
    grid: &Grid,
    cells: Vec<Cell>,
    row: usize,
    cursor_row: usize,
    cursor_col: usize,
) -> impl IntoElement {
    let mut spans: Vec<(String, u32, u32, Flags)> = Vec::new();
    for (col, cell) in cells.iter().enumerate() {
        let is_cursor = row == cursor_row && col == cursor_col;
        let is_selected = grid.is_selected(col, row);
        let (mut fg, mut bg) = (resolve_color(cell.fg, true), resolve_color(cell.bg, false));
        if cell.flags.contains(Flags::INVERSE) ^ is_cursor ^ is_selected {
            std::mem::swap(&mut fg, &mut bg);
        }
        match spans.last_mut() {
            Some((text, span_fg, span_bg, span_flags))
                if *span_fg == fg && *span_bg == bg && *span_flags == cell.flags =>
            {
                text.push(cell.c);
            }
            _ => spans.push((cell.c.to_string(), fg, bg, cell.flags)),
        }
    }

    h_flex()
        .w_full()
        .children(spans.into_iter().map(|(text, fg, bg, flags)| {
            let mut span = div().text_color(rgb(fg)).bg(rgb(bg)).child(text);
            if flags.contains(Flags::BOLD) {
                span = span.font_weight(gpui_kit::FontWeight::BOLD);
            }
            if flags.contains(Flags::ITALIC) {
                span = span.italic();
            }
            if flags.intersects(Flags::UNDERLINE | Flags::DOUBLE_UNDERLINE) {
                span = span.underline();
            }
            if flags.contains(Flags::STRIKEOUT) {
                span = span.line_through();
            }
            span
        }))
}

/// Resolves an `alacritty_terminal` color to a fixed RGB value. `is_fg`
/// picks the right default for the two terminal-scheme-relative named
/// colors (`Foreground`/`Background`) - everything else (the 16 ANSI
/// colors, the 256-color palette, true-color spec values) is fixed
/// regardless of foreground/background position.
fn resolve_color(color: Color, is_fg: bool) -> u32 {
    match color {
        Color::Spec(Rgb { r, g, b }) => rgb_to_u32(r, g, b),
        Color::Named(NamedColor::Foreground) => DEFAULT_FOREGROUND,
        Color::Named(NamedColor::Background) => DEFAULT_BACKGROUND,
        Color::Named(named) => named_color(named).unwrap_or(if is_fg {
            DEFAULT_FOREGROUND
        } else {
            DEFAULT_BACKGROUND
        }),
        Color::Indexed(index) => indexed_color(index),
    }
}

fn rgb_to_u32(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

/// The standard 16-color ANSI palette (xterm's defaults).
fn named_color(color: NamedColor) -> Option<u32> {
    Some(match color {
        NamedColor::Black => 0x000000,
        NamedColor::Red => 0xCD0000,
        NamedColor::Green => 0x00CD00,
        NamedColor::Yellow => 0xCDCD00,
        NamedColor::Blue => 0x0000EE,
        NamedColor::Magenta => 0xCD00CD,
        NamedColor::Cyan => 0x00CDCD,
        NamedColor::White => 0xE5E5E5,
        NamedColor::BrightBlack => 0x7F7F7F,
        NamedColor::BrightRed => 0xFF0000,
        NamedColor::BrightGreen => 0x00FF00,
        NamedColor::BrightYellow => 0xFFFF00,
        NamedColor::BrightBlue => 0x5C5CFF,
        NamedColor::BrightMagenta => 0xFF00FF,
        NamedColor::BrightCyan => 0x00FFFF,
        NamedColor::BrightWhite => 0xFFFFFF,
        _ => return None,
    })
}

/// The xterm 256-color palette: 0-15 mirror the named ANSI colors, 16-231
/// are a 6x6x6 color cube, 232-255 are a grayscale ramp.
fn indexed_color(index: u8) -> u32 {
    const ANSI_16: [u32; 16] = [
        0x000000, 0xCD0000, 0x00CD00, 0xCDCD00, 0x0000EE, 0xCD00CD, 0x00CDCD, 0xE5E5E5, 0x7F7F7F,
        0xFF0000, 0x00FF00, 0xFFFF00, 0x5C5CFF, 0xFF00FF, 0x00FFFF, 0xFFFFFF,
    ];
    match index {
        0..=15 => ANSI_16[index as usize],
        16..=231 => {
            let i = index - 16;
            let levels = [0u32, 95, 135, 175, 215, 255];
            let r = levels[(i / 36) as usize];
            let g = levels[((i / 6) % 6) as usize];
            let b = levels[(i % 6) as usize];
            rgb_to_u32(r as u8, g as u8, b as u8)
        }
        232..=255 => {
            let level = 8 + (index - 232) as u32 * 10;
            rgb_to_u32(level as u8, level as u8, level as u8)
        }
    }
}
