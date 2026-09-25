//! An agent's three decoration collections, and keeping them in step with
//! its buffer.

use gpui_kit::App;
use gpui_kit::Entity;
use gpui_kit::component::input::TextDecoration;
use gpui_kit::component::input::TextDecorationCollection;

use crate::composer_scan::Edit;
use crate::composer_scan::Span;
use crate::composer_scan::rescan;
use crate::composer_scan::scan;
use crate::composer_style::treatment::Layer;
use crate::composer_style::treatment::Palette;
use crate::composer_style::treatment::treatment;
use crate::workspace_window::panel::prompt::PanelInputState;

/// The styling state for one agent's composer.
///
/// Holds the collections, the buffer they describe, and the palette they
/// were painted from - the three things that decide whether anything has
/// to be done.
pub(crate) struct ComposerStyling {
    /// One per [`Layer`], in `Layer::ALL` order, which is the order they
    /// were created in and therefore their precedence.
    collections: Vec<TextDecorationCollection>,
    /// The buffer the spans below describe.
    text:        String,
    spans:       Vec<Span>,
    palette:     Palette,
}

impl ComposerStyling {
    /// Creates the three collections for `input` and paints what is
    /// already in it.
    ///
    /// A composer built around a restored draft is styled here, with no
    /// edit required to trigger it (`panel-rich-input`, "A restored draft
    /// is styled").
    pub(crate) fn new(input: &Entity<PanelInputState>, attachments: &[&str], palette: Palette,
                      cx: &mut App)
                      -> Self {
        let text = input.read(cx).value().to_string();
        let spans = scan(&text, attachments);
        // Creation order is precedence, so this loop is load-bearing and
        // `Layer::ALL` is what defines it.
        let collections = Layer::ALL.iter()
                                    .map(|layer| {
                                        let decorations = decorations(&spans, *layer, &palette);
                                        input.update(cx, |state, cx| {
                                                 state.create_decorations_collection(decorations,
                                                                                     cx)
                                             })
                                    })
                                    .collect();
        Self { collections,
               text,
               spans,
               palette }
    }

    /// Restyles after the buffer changed, rescanning only what the change
    /// can have affected.
    ///
    /// Returns whether anything was repainted. `InputEvent::Change` fires
    /// for every way text arrives - typing, paste, undo, redo, cut, a drag
    /// of text, and the lookup's own insertion - so this one path covers
    /// all of them, which is what "survives every way the composer
    /// changes" asks for.
    pub(crate) fn on_change(&mut self, input: &Entity<PanelInputState>, attachments: &[&str],
                            cx: &mut App)
                            -> bool {
        let text = input.read(cx).value().to_string();
        if text == self.text {
            return false;
        }
        // `InputEvent::Change` carries no payload, so the edit is
        // recovered by comparing the buffers. That is a byte scan, not a
        // parse: it costs a memcmp of the common prefix and suffix, and
        // buys the scanner the bound it could not otherwise have.
        let edit = edit_between(&self.text, &text);
        self.spans = rescan(&text, attachments, &self.spans, &edit);
        self.text = text;
        self.paint(cx);
        true
    }

    /// Repaints in `palette` if it is not the one already on screen.
    ///
    /// The spans do not depend on the theme, so an appearance switch is a
    /// repaint and never a rescan - which is what lets it hang off the
    /// re-render GPUI already does.
    pub(crate) fn on_palette(&mut self, palette: Palette, cx: &mut App) -> bool {
        if palette == self.palette {
            return false;
        }
        self.palette = palette;
        self.paint(cx);
        true
    }

    /// Recomputes every span from scratch, for a change this cannot
    /// attribute to an edit - the attachment table gaining or losing a
    /// row, say.
    pub(crate) fn reset(&mut self, input: &Entity<PanelInputState>, attachments: &[&str],
                        cx: &mut App) {
        self.text = input.read(cx).value().to_string();
        self.spans = scan(&self.text, attachments);
        self.paint(cx);
    }

    /// The spans currently painted.
    // UNWIRED(#396): the buffer-decides reconciliation reads this to find
    // where an agent's chips are. That is task group 7 of the
    // rich-prompt-composer change; until it lands only tests ask.
    #[allow(dead_code)]
    pub(crate) fn spans(&self) -> &[Span] {
        &self.spans
    }

    /// Hands every collection the decorations for its own layer.
    fn paint(&self, cx: &mut App) {
        for (collection, layer) in self.collections.iter().zip(Layer::ALL) {
            collection.set(decorations(&self.spans, layer, &self.palette), cx);
        }
    }
}

/// The decorations for one layer, resolved against `palette`.
pub(crate) fn decorations(spans: &[Span], layer: Layer, palette: &Palette) -> Vec<TextDecoration> {
    spans.iter()
         .filter(|span| Layer::of(span.construct) == layer)
         .map(|span| TextDecoration::new(span.range.clone(), treatment(span.construct, palette)))
         .collect()
}

/// The single replacement that turns `old` into `new`.
///
/// Not the minimal edit in any interesting sense - it is the common prefix
/// and suffix trimmed off - but that is exactly what a keystroke, a paste
/// or a cut produces, and an undo restoring a distant region reports as a
/// wider edit rather than a wrong one.
pub(crate) fn edit_between(old: &str, new: &str) -> Edit {
    let prefix = common_prefix(old, new);
    let suffix = common_suffix(&old[prefix..], &new[prefix..]);
    Edit { replaced: prefix..old.len() - suffix,
           inserted: new.len() - prefix - suffix, }
}

/// How many leading bytes `old` and `new` share, rounded down to a
/// character boundary in both.
fn common_prefix(old: &str, new: &str) -> usize {
    let mut shared = old.as_bytes()
                        .iter()
                        .zip(new.as_bytes())
                        .take_while(|(a, b)| a == b)
                        .count();
    while shared > 0 && !(old.is_char_boundary(shared) && new.is_char_boundary(shared)) {
        shared -= 1;
    }
    shared
}

/// How many trailing bytes `old` and `new` share, rounded down to a
/// character boundary in both.
fn common_suffix(old: &str, new: &str) -> usize {
    let mut shared = old.as_bytes()
                        .iter()
                        .rev()
                        .zip(new.as_bytes().iter().rev())
                        .take_while(|(a, b)| a == b)
                        .count();
    while shared > 0
          && !(old.is_char_boundary(old.len() - shared) && new.is_char_boundary(new.len() - shared))
    {
        shared -= 1;
    }
    shared
}
