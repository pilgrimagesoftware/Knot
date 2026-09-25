//! What each construct looks like.
//!
//! Every treatment is resolved from `cx.theme()` when the decorations are
//! built, never stored as a constant, so a change of system appearance
//! produces new styles from the same spans
//! (`panel-rich-input`, "Styling reads correctly in both appearances").
//!
//! **No treatment distinguishes itself by hue alone.** Each carries a
//! weight, a slant, a background or an underline as well, so the styling
//! survives a low-contrast theme and does not rely on colour vision. That
//! is a requirement rather than a preference, and
//! `tests::every_treatment_carries_more_than_a_hue` is what holds it.

use gpui_kit::App;
use gpui_kit::FontStyle;
use gpui_kit::FontWeight;
use gpui_kit::HighlightStyle;
use gpui_kit::Hsla;
use gpui_kit::UnderlineStyle;
use gpui_kit::component::ActiveTheme;
use gpui_kit::px;

use crate::composer_scan::Construct;

/// Which decoration collection a construct belongs to.
///
/// The three are created in this order and `gpui-base` resolves a
/// contested property in favour of the first, so the order is the
/// precedence: a chip is exact and must not be overridden, a token is a
/// bounded run, and markdown is the broadest and most likely to overlap
/// the other two.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Layer {
    Attachment,
    Token,
    Markdown,
}

impl Layer {
    /// The three, in creation order.
    pub(crate) const ALL: [Self; 3] = [Self::Attachment, Self::Token, Self::Markdown];

    /// The layer `construct` is painted in.
    pub(crate) fn of(construct: Construct) -> Self {
        match construct {
            Construct::Attachment => Self::Attachment,
            Construct::SlashToken | Construct::Mention => Self::Token,
            Construct::Emphasis
            | Construct::Strong
            | Construct::InlineCode
            | Construct::CodeFence
            | Construct::Heading
            | Construct::ListMarker
            | Construct::BlockQuote
            | Construct::Link => Self::Markdown,
        }
    }
}

/// The handful of theme colours the treatments draw from, lifted out of
/// the theme once per restyle.
///
/// A struct rather than the theme itself for two reasons: the theme is
/// around a hundred fields and this runs per keystroke, and a palette can
/// be built by hand, so [`treatment`] is checkable without a window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Palette {
    pub(crate) foreground:           Hsla,
    pub(crate) muted:                Hsla,
    pub(crate) muted_foreground:     Hsla,
    pub(crate) accent_foreground:    Hsla,
    pub(crate) primary:              Hsla,
    pub(crate) secondary:            Hsla,
    pub(crate) secondary_foreground: Hsla,
    /// Not used by any treatment - it is what says whether a repaint is
    /// needed at all when the system appearance flips.
    pub(crate) is_dark:              bool,
}

impl Palette {
    /// The active theme's colours.
    ///
    /// Takes the app rather than a context so a caller holding either can
    /// pass it: `Context` derefs to `App`, and `ActiveTheme` is
    /// implemented on `App`.
    pub(crate) fn of(app: &App) -> Self {
        let theme = app.theme();
        Self { foreground:           theme.foreground,
               muted:                theme.muted,
               muted_foreground:     theme.muted_foreground,
               accent_foreground:    theme.accent_foreground,
               primary:              theme.primary,
               secondary:            theme.secondary,
               secondary_foreground: theme.secondary_foreground,
               is_dark:              theme.is_dark(), }
    }
}

/// How `construct` is drawn against `palette`.
pub(crate) fn treatment(construct: Construct, palette: &Palette) -> HighlightStyle {
    match construct {
        // A chip: a bounded, visibly self-contained run. The background is
        // what makes it read as one object rather than as coloured words,
        // which is the whole difference between a chip and a mention.
        Construct::Attachment => HighlightStyle { color: Some(palette.secondary_foreground),
                                                  background_color: Some(palette.secondary),
                                                  font_weight: Some(FontWeight::MEDIUM),
                                                  ..Default::default() },
        // Tokens are the composer's own vocabulary, so they take a weight
        // rather than a background: a token is part of the sentence, not
        // an object sitting in it.
        Construct::SlashToken => HighlightStyle { color: Some(palette.accent_foreground),
                                                  font_weight: Some(FontWeight::SEMIBOLD),
                                                  ..Default::default() },
        // A mention differs from a slash command by colour and by slant,
        // so the two are not one hue apart.
        Construct::Mention => HighlightStyle { color: Some(palette.primary),
                                               font_style: Some(FontStyle::Italic),
                                               font_weight: Some(FontWeight::SEMIBOLD),
                                               ..Default::default() },
        Construct::Emphasis => HighlightStyle { font_style: Some(FontStyle::Italic),
                                                ..Default::default() },
        Construct::Strong => HighlightStyle { font_weight: Some(FontWeight::BOLD),
                                              ..Default::default() },
        // Code cannot change font family from a `HighlightStyle`, so the
        // background is what says "this is quoted" - which is also what
        // keeps it legible when a theme's muted colours sit close to the
        // foreground.
        Construct::InlineCode | Construct::CodeFence => {
            HighlightStyle { color: Some(palette.muted_foreground),
                             background_color: Some(palette.muted),
                             ..Default::default() }
        }
        Construct::Heading => HighlightStyle { color: Some(palette.foreground),
                                               font_weight: Some(FontWeight::EXTRA_BOLD),
                                               ..Default::default() },
        Construct::ListMarker => HighlightStyle { color: Some(palette.accent_foreground),
                                                  font_weight: Some(FontWeight::BOLD),
                                                  ..Default::default() },
        Construct::BlockQuote => HighlightStyle { color: Some(palette.muted_foreground),
                                                  font_style: Some(FontStyle::Italic),
                                                  ..Default::default() },
        // The one treatment whose second signal is an underline rather
        // than a weight or a slant. A link that is also bold or italic
        // collides with the emphasis it is often written inside; an
        // underline is the conventional mark, and is no more a hue than
        // the others are.
        Construct::Link => HighlightStyle { color: Some(palette.primary),
                                            underline: Some(UnderlineStyle { thickness: px(1.),
                                                                             color:     None,
                                                                             wavy:      false, }),
                                            ..Default::default() },
    }
}
