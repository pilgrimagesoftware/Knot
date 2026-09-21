//! What a font picker's row should read, and in which family to draw it.
//!
//! Pure: it takes the configured name and size plus the list of families the
//! text system can actually resolve, and returns the decision. Split out of
//! the appearance pane so the rule can be asserted without a window, a text
//! system, or an installed font.

/// What a font picker's label should read, and the family it should be drawn
/// in.
pub(crate) struct FontLabel {
    /// The family to draw the label in, or `None` to leave the settings
    /// window's own face in place because the configured family does not
    /// resolve. Never the substitute's name: the row goes on naming what is
    /// persisted, and marks itself instead.
    pub(crate) family: Option<gpui_kit::SharedString>,
    pub(crate) text:   String,
}

/// The decision behind a font picker's label, split out from
/// `font_picker_button` so it can be tested without a window.
///
/// `resolvable` is `cx.text_system().all_font_names()` - the same question
/// `terminal_font_family` asks, so the preview and the terminal cannot
/// disagree about whether a family exists. A family that is not in that list
/// is marked rather than substituted: a label drawn in one face while naming
/// another would report a fault as a preference. The persisted value is not
/// touched either way, since a font missing today may be installed tomorrow.
pub(crate) fn font_label(name: &str, size: f64, resolvable: &[String]) -> FontLabel {
    let text = format!("{name}, {size:.0}pt");
    if resolvable.iter().any(|candidate| candidate == name) {
        FontLabel { family: Some(name.to_string().into()),
                    text }
    }
    else {
        FontLabel { family: None,
                    text:   format!("{text} ({})",
                                    knot_core::l10n::t("settings.font_unavailable")), }
    }
}
