//! The treatments, the layering, and the edit recovery that feeds them.
//!
//! None of these needs a window either: a [`Palette`] can be built by
//! hand, so what a construct looks like is checkable without a theme
//! behind it. What they cannot check is whether it *reads* well, which is
//! why the change also asks for a walkthrough in a debug build.

use gpui_kit::Hsla;
use gpui_kit::hsla;

use crate::composer_scan::Construct;
use crate::composer_style::layers::edit_between;
use crate::composer_style::treatment::Layer;
use crate::composer_style::treatment::Palette;
use crate::composer_style::treatment::treatment;

/// Every construct, so a new one cannot be added without deciding what it
/// looks like and which layer it belongs to.
const ALL: [Construct; 11] = [Construct::Attachment,
                              Construct::SlashToken,
                              Construct::Mention,
                              Construct::Emphasis,
                              Construct::Strong,
                              Construct::InlineCode,
                              Construct::CodeFence,
                              Construct::Heading,
                              Construct::ListMarker,
                              Construct::BlockQuote,
                              Construct::Link];

/// A palette whose colours are all distinct, so a test that compares two
/// treatments is comparing the treatments rather than a theme's
/// coincidences.
fn palette(is_dark: bool) -> Palette {
    let shade =
        |index: f32| -> Hsla { hsla(index / 12., 0.5, if is_dark { 0.7 } else { 0.3 }, 1.) };
    Palette { foreground: shade(1.),
              muted: shade(2.),
              muted_foreground: shade(3.),
              accent_foreground: shade(4.),
              primary: shade(5.),
              secondary: shade(6.),
              secondary_foreground: shade(7.),
              is_dark }
}

/// `panel-rich-input`: "A treatment SHALL NOT be the only thing
/// distinguishing two constructs when it relies on hue alone - weight,
/// slant, or a background SHALL carry the distinction as well."
///
/// A link's second signal is an underline rather than one of the three the
/// requirement names. That is a deliberate reading: the point is that
/// colour vision is never the only channel, and an underline is as much a
/// non-hue signal as a weight is. Bold or italic would have collided with
/// the emphasis links are usually written inside.
#[test]
fn every_treatment_carries_more_than_a_hue() {
    for construct in ALL {
        let style = treatment(construct, &palette(false));
        let non_hue = style.font_weight.is_some()
                      || style.font_style.is_some()
                      || style.background_color.is_some()
                      || style.underline.is_some();
        assert!(non_hue,
                "{construct:?} is distinguished by colour alone, so it disappears for a reader \
                 who cannot separate its hue from prose");
    }
}

/// Prose is the absence of a treatment, so every construct has to differ
/// from an empty style in some way - a treatment that sets nothing is
/// invisible.
#[test]
fn no_treatment_is_empty() {
    for construct in ALL {
        assert_ne!(treatment(construct, &palette(false)),
                   gpui_kit::HighlightStyle::default(),
                   "{construct:?} would be drawn exactly like prose");
    }
}

/// Constructs the user is meant to tell apart have to actually differ.
/// Inline code and a fenced block are deliberately one treatment - the
/// spec asks for that - so they are the single permitted pair.
#[test]
fn treatments_are_distinct_except_for_the_two_code_forms() {
    let palette = palette(false);
    for (index, first) in ALL.iter().enumerate() {
        for second in &ALL[index + 1..] {
            let code_pair = matches!((first, second),
                                     (Construct::InlineCode, Construct::CodeFence));
            if code_pair {
                assert_eq!(treatment(*first, &palette),
                           treatment(*second, &palette),
                           "a fenced block and a code span are one code treatment");
                continue;
            }
            assert_ne!(treatment(*first, &palette),
                       treatment(*second, &palette),
                       "{first:?} and {second:?} are drawn identically");
        }
    }
}

/// The treatments come from the theme rather than from constants, so a
/// change of appearance produces different styles from the same spans -
/// which is what lets an appearance switch be a repaint and not a rescan.
#[test]
fn the_appearance_changes_the_treatments() {
    let light = palette(false);
    let dark = palette(true);
    assert_ne!(light, dark,
               "the fixture has to differ, or this proves nothing");

    let coloured = ALL.iter()
                      .filter(|construct| treatment(**construct, &light).color.is_some())
                      .count();
    assert!(coloured > 0);

    for construct in ALL {
        let in_light = treatment(construct, &light);
        let in_dark = treatment(construct, &dark);
        if in_light.color.is_some() || in_light.background_color.is_some() {
            assert_ne!(in_light, in_dark,
                       "{construct:?} draws the same in both appearances, so it was not resolved \
                        from the theme");
        }
    }
}

/// Precedence is creation order, so the mapping from construct to layer is
/// the thing that decides which treatment wins an overlap.
#[test]
fn each_construct_lands_in_the_layer_its_precedence_needs() {
    assert_eq!(Layer::of(Construct::Attachment), Layer::Attachment);
    assert_eq!(Layer::of(Construct::SlashToken), Layer::Token);
    assert_eq!(Layer::of(Construct::Mention), Layer::Token);
    for construct in ALL {
        let markdown =
            !matches!(construct,
                      Construct::Attachment | Construct::SlashToken | Construct::Mention);
        if markdown {
            assert_eq!(Layer::of(construct), Layer::Markdown, "{construct:?}");
        }
    }
}

/// A chip must not be overridden by anything, and markdown is the broadest
/// and so must lose - `Layer::ALL` is the order the collections are
/// created in, and creation order is what `gpui-base` resolves by.
#[test]
fn the_layers_are_ordered_chip_first_and_markdown_last() {
    assert_eq!(Layer::ALL,
               [Layer::Attachment, Layer::Token, Layer::Markdown]);
}

#[test]
fn an_inserted_character_is_recovered_as_a_one_byte_edit() {
    let edit = edit_between("ab", "axb");
    assert_eq!(edit.replaced, 1..1);
    assert_eq!(edit.inserted, 1);
}

#[test]
fn a_deletion_is_recovered_as_a_replacement_by_nothing() {
    let edit = edit_between("axb", "ab");
    assert_eq!(edit.replaced, 1..2);
    assert_eq!(edit.inserted, 0);
}

#[test]
fn a_replacement_is_recovered_as_both() {
    let edit = edit_between("keep OLD tail", "keep NEW tail");
    assert_eq!(edit.replaced, 5..8);
    assert_eq!(edit.inserted, 3);
}

#[test]
fn an_unchanged_buffer_is_an_empty_edit() {
    let edit = edit_between("same", "same");
    assert_eq!(edit.inserted, 0);
    assert!(edit.replaced.is_empty());
}

/// The recovered edit is read as byte offsets, so landing it inside a
/// character would panic the moment the scanner sliced the buffer.
#[test]
fn an_edit_beside_multibyte_text_stays_on_character_boundaries() {
    for (old, new) in [("héllo", "héllo!"), ("héllo", "hllo"), ("🙂🙂", "🙂x🙂")] {
        let edit = edit_between(old, new);
        assert!(old.is_char_boundary(edit.replaced.start)
                && old.is_char_boundary(edit.replaced.end),
                "{old:?} -> {new:?} produced {:?}, which splits a character",
                edit.replaced);
        assert!(new.is_char_boundary(edit.replaced.start));
    }
}

/// The recovered edit has to describe the change it came from, or the
/// incremental scan is rebuilt from a lie. Applying it to the old buffer
/// must reproduce the new one.
#[test]
fn the_recovered_edit_reproduces_the_new_buffer() {
    let cases = [("", "hello"),
                 ("hello", ""),
                 ("a *word", "a *word*"),
                 ("```\ncode\n```", "```\ncode\n"),
                 ("one\ntwo\nthree", "one\n2\nthree"),
                 ("héllo wörld", "héllo there wörld")];
    for (old, new) in cases {
        let edit = edit_between(old, new);
        let mut rebuilt = String::new();
        rebuilt.push_str(&old[..edit.replaced.start]);
        rebuilt.push_str(&new[edit.replaced.start..edit.replaced.start + edit.inserted]);
        rebuilt.push_str(&old[edit.replaced.end..]);
        assert_eq!(rebuilt, new, "{old:?} -> {new:?} was recovered as {edit:?}");
    }
}
