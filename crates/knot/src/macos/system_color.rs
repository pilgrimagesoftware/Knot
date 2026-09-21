//! Resolves the macOS system palette - the user's accent color and the
//! dynamic neutrals - into GPUI colors, so the app's chrome follows the
//! platform instead of a fixed web palette.
//!
//! Contract: `openspec/specs/acp-panel-ui/spec.md` - "Panel chrome adopts the
//! macOS system palette".
//!
//! Only the AppKit queries are `cfg`-gated; the derivations below them are
//! ordinary arithmetic on GPUI colors, so they stay testable on any target
//! and callers never need a `cfg` of their own - [`resolve`] hands back the
//! fixed palette everywhere AppKit is not available.

use gpui_kit::{Hsla, Rgba, rgb};

/// The fixed accent family the app painted before it read the system: the
/// Tailwind blue 500/600/700 triple. Still what non-macOS targets get, so
/// their chrome is unchanged.
const FIXED_ACCENT: u32 = 0x3B82F6;
const FIXED_ACCENT_HOVER: u32 = 0x2563EB;
const FIXED_ACCENT_ACTIVE: u32 = 0x1D4ED8;

/// How far [`accent_states`] moves lightness for the hover and active
/// states. Chosen to land near the fixed triple's own spacing for a
/// default-blue accent, so the interaction states keep the weight they had.
const HOVER_LIGHTNESS_STEP: f32 = 0.06;
const ACTIVE_LIGHTNESS_STEP: f32 = 0.12;

/// Above this relative luminance an accent is "light" and needs dark text.
/// 0.55 rather than the midpoint: white-on-color stays legible further down
/// the scale than black-on-color does up it.
const LIGHT_ACCENT_LUMINANCE: f32 = 0.55;

/// The dark foreground for a light accent - near-black rather than black,
/// so a mid-tone accent does not get a hard, printed-looking contrast.
const DARK_FOREGROUND: u32 = 0x1A1A1A;

/// The chrome colors resolved from the platform.
///
/// The accent family is always present (the fixed palette stands in where
/// the system has no answer); `neutrals` is `None` when there is no system
/// appearance to read, which is the signal to leave the theme's own
/// background and border colors alone.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SystemPalette {
    pub(crate) accent:            Hsla,
    pub(crate) accent_hover:      Hsla,
    pub(crate) accent_active:     Hsla,
    /// Text drawn on top of `accent`, picked for contrast against it.
    pub(crate) accent_foreground: Hsla,
    pub(crate) neutrals:          Option<SystemNeutrals>,
}

/// The appearance-tracking neutrals: surfaces and separators that should
/// follow light/dark rather than being painted at a fixed value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SystemNeutrals {
    /// `NSColor.windowBackgroundColor` - the root window surface.
    pub(crate) window_background:  Hsla,
    /// `NSColor.controlBackgroundColor` - cards, inputs, bubbles.
    pub(crate) control_background: Hsla,
    /// `NSColor.separatorColor` - card borders and dividers.
    pub(crate) separator:          Hsla,
}

impl SystemPalette {
    /// The palette the app painted before it consulted the platform. Used
    /// off macOS, and on macOS whenever AppKit cannot be reached.
    pub(crate) fn fixed() -> Self {
        Self { accent:            rgb(FIXED_ACCENT).into(),
               accent_hover:      rgb(FIXED_ACCENT_HOVER).into(),
               accent_active:     rgb(FIXED_ACCENT_ACTIVE).into(),
               accent_foreground: gpui_kit::white(),
               neutrals:          None, }
    }

    /// Builds the full palette from a single system accent, deriving the
    /// interaction states and the contrasting foreground from it.
    fn from_accent(accent: Hsla, neutrals: Option<SystemNeutrals>) -> Self {
        let (accent_hover, accent_active) = accent_states(accent);
        Self { accent,
               accent_hover,
               accent_active,
               accent_foreground: contrast_foreground(accent),
               neutrals }
    }
}

/// Resolves the platform palette at the current effective appearance.
///
/// Off macOS - and on macOS off the main thread, where AppKit must not be
/// touched - this is [`SystemPalette::fixed`], so every call site can stay
/// unconditional.
pub(crate) fn resolve() -> SystemPalette {
    #[cfg(target_os = "macos")]
    {
        appkit::resolve().unwrap_or_else(SystemPalette::fixed)
    }
    #[cfg(not(target_os = "macos"))]
    {
        SystemPalette::fixed()
    }
}

/// The hover and active shades for `accent`.
///
/// `controlAccentColor` is a single color, but the components need two more
/// interaction states, so they are stepped off the accent's own lightness
/// rather than invented: darker as the control is pressed harder, which
/// reads as "pressed" on any hue. A very dark accent has no room left to go
/// down, so the steps move *up* instead and the states stay distinguishable
/// at either end of the scale.
fn accent_states(accent: Hsla) -> (Hsla, Hsla) {
    let downward = accent.l > ACTIVE_LIGHTNESS_STEP;
    let step = |amount: f32| {
        let l = if downward {
            accent.l - amount
        }
        else {
            accent.l + amount
        };
        Hsla { l: l.clamp(0., 1.),
               ..accent }
    };
    (step(HOVER_LIGHTNESS_STEP), step(ACTIVE_LIGHTNESS_STEP))
}

/// White or near-black, whichever reads against `background`.
///
/// Uses the standard sRGB luminance coefficients on the color's own
/// channels, so an accent the user picked at any hue gets the foreground
/// that is actually legible on it rather than a fixed white that disappears
/// on yellow.
fn contrast_foreground(background: Hsla) -> Hsla {
    if relative_luminance(background) > LIGHT_ACCENT_LUMINANCE {
        rgb(DARK_FOREGROUND).into()
    }
    else {
        gpui_kit::white()
    }
}

/// Relative luminance in the 0.0 (black) to 1.0 (white) range, by the sRGB
/// coefficients. Deliberately the simple gamma-naive form: it only has to
/// order colors either side of one threshold, not model perception.
fn relative_luminance(color: Hsla) -> f32 {
    let rgba: Rgba = color.into();
    0.2126 * rgba.r + 0.7152 * rgba.g + 0.0722 * rgba.b
}

/// The AppKit side: reads `NSColor`'s dynamic system colors and converts
/// them to GPUI colors.
#[cfg(target_os = "macos")]
mod appkit {
    use gpui_kit::{Hsla, Rgba};
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSColor, NSColorSpace};

    use super::{SystemNeutrals, SystemPalette};

    /// Reads the system colors, or `None` off the main thread - AppKit is
    /// main-thread-only and the palette is re-resolved on every appearance
    /// change anyway, so a missed read costs nothing.
    ///
    /// Dynamic colors resolve against `NSAppearance.currentDrawingAppearance`,
    /// which outside a drawing operation is the application's effective
    /// appearance - exactly the appearance the theme is being built for. No
    /// appearance is pushed around the queries, so this stays clear of the
    /// deprecated `+setCurrentAppearance:`.
    pub(super) fn resolve() -> Option<SystemPalette> {
        // The marker is the "we are on the main thread" proof, not an
        // argument any of these class methods take.
        let _mtm = MainThreadMarker::new()?;
        let accent = to_hsla(&NSColor::controlAccentColor())?;
        let neutrals = neutrals();
        Some(SystemPalette::from_accent(accent, neutrals))
    }

    /// The dynamic neutrals, all-or-nothing: a partial set would mix system
    /// surfaces with theme borders and look like neither appearance.
    fn neutrals() -> Option<SystemNeutrals> {
        Some(SystemNeutrals { window_background:  to_hsla(&NSColor::windowBackgroundColor())?,
                              control_background: to_hsla(&NSColor::controlBackgroundColor())?,
                              separator:          to_hsla(&NSColor::separatorColor())?, })
    }

    /// Resolves a (possibly dynamic, possibly catalog-backed) `NSColor` into
    /// GPUI's color space.
    ///
    /// `colorUsingColorSpace:` is what does the resolving - it flattens the
    /// color to concrete sRGB channels for the current appearance, and
    /// returns `None` for a color that has no sRGB representation (patterns,
    /// for instance), which is why the component reads below it are safe.
    fn to_hsla(color: &NSColor) -> Option<Hsla> {
        let srgb = color.colorUsingColorSpace(&NSColorSpace::sRGBColorSpace())?;
        Some(Hsla::from(Rgba { r: srgb.redComponent() as f32,
                               g: srgb.greenComponent() as f32,
                               b: srgb.blueComponent() as f32,
                               a: srgb.alphaComponent() as f32, }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Within a tolerance, since the trip through HSLA is lossy.
    fn assert_close(left: Hsla, right: Hsla) {
        let (left, right): (Rgba, Rgba) = (left.into(), right.into());
        for (l, r) in [(left.r, right.r), (left.g, right.g), (left.b, right.b)] {
            assert!((l - r).abs() < 0.01, "{left:?} != {right:?}");
        }
    }

    #[test]
    fn fixed_palette_is_the_pre_system_accent_family() {
        let palette = SystemPalette::fixed();
        assert_close(palette.accent, rgb(0x3B82F6).into());
        assert_close(palette.accent_hover, rgb(0x2563EB).into());
        assert_close(palette.accent_active, rgb(0x1D4ED8).into());
        assert_eq!(palette.accent_foreground, gpui_kit::white());
        assert_eq!(palette.neutrals, None);
    }

    #[test]
    fn accent_states_darken_a_mid_tone_accent() {
        let accent = Hsla { h: 0.6,
                            s: 0.9,
                            l: 0.5,
                            a: 1.0, };
        let (hover, active) = accent_states(accent);
        assert!(hover.l < accent.l);
        assert!(active.l < hover.l);
        // Hue and saturation are the accent's, so the states read as the
        // same color rather than a second tint.
        assert_eq!((hover.h, hover.s), (accent.h, accent.s));
        assert_eq!((active.h, active.s), (accent.h, accent.s));
    }

    #[test]
    fn accent_states_lighten_a_near_black_accent() {
        let accent = Hsla { h: 0.1,
                            s: 0.5,
                            l: 0.04,
                            a: 1.0, };
        let (hover, active) = accent_states(accent);
        assert!(hover.l > accent.l, "hover must stay distinguishable");
        assert!(active.l > hover.l, "active must stay distinguishable");
    }

    #[test]
    fn accent_states_stay_in_range() {
        for l in [0.0, 0.01, 0.5, 0.99, 1.0] {
            let accent = Hsla { h: 0.3,
                                s: 1.0,
                                l,
                                a: 1.0 };
            let (hover, active) = accent_states(accent);
            for state in [hover, active] {
                assert!((0.0..=1.0).contains(&state.l),
                        "lightness {} out of range",
                        state.l);
            }
        }
    }

    #[test]
    fn a_dark_accent_takes_white_text() {
        // The default macOS blue.
        assert_eq!(contrast_foreground(rgb(0x0A84FF).into()), gpui_kit::white());
        assert_eq!(contrast_foreground(gpui_kit::black()), gpui_kit::white());
    }

    #[test]
    fn a_light_accent_takes_dark_text() {
        // The macOS yellow accent, the case a fixed white foreground loses.
        assert_close(contrast_foreground(rgb(0xFFC600).into()),
                     rgb(DARK_FOREGROUND).into());
        assert_close(contrast_foreground(gpui_kit::white()),
                     rgb(DARK_FOREGROUND).into());
    }

    #[test]
    fn luminance_orders_black_below_white() {
        assert!(relative_luminance(gpui_kit::black()) < relative_luminance(gpui_kit::white()));
        assert!(relative_luminance(gpui_kit::white()) > 0.99);
    }

    #[test]
    fn a_system_accent_drives_the_whole_family() {
        let accent: Hsla = rgb(0xFF453A).into();
        let palette = SystemPalette::from_accent(accent, None);
        assert_eq!(palette.accent, accent);
        assert_eq!(palette.accent_hover, accent_states(accent).0);
        assert_eq!(palette.accent_active, accent_states(accent).1);
        assert_eq!(palette.accent_foreground, contrast_foreground(accent));
    }

    /// Off macOS the resolver must not move a single color, or the
    /// non-Apple chrome would drift from what it renders today.
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn non_macos_resolves_to_the_fixed_palette() {
        assert_eq!(resolve(), SystemPalette::fixed());
    }
}
