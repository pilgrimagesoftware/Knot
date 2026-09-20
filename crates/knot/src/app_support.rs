use super::*;
pub(crate) const CHECK_INBOX_PROMPT: &str = "Check your inbox for questions or instructions from other agents. Update your status and immediately execute what is being asked without confirmation.";
pub(crate) type AwaitingInputQueue = Arc<Mutex<Vec<(Uuid, Option<String>)>>>;

/// Which setting a font panel session is editing. Plain data, referenced
/// from platform-independent UI code (button labels/handlers); only the
/// panel-driving logic that reads/writes it is macOS-only, in
/// `native_font_panel` below.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FontPanelTarget {
    Ui,
    Title,
    Terminal,
}

/// Drives the real macOS font panel (`NSFontPanel`) for the terminal font
/// picker, since the user wants the system chooser rather than an in-app
/// dropdown. `NSFontManager.selectedFont` updates live as the user clicks
/// around the panel, so we just poll it from GPUI (see [`poll_selection`])
/// instead of relying on the `changeFont:` target/action message - AppKit's
/// own responder chain (e.g. text views taking first responder) can steal
/// that target, but the property read is unaffected either way.
#[cfg(target_os = "macos")]
pub(crate) mod native_font_panel {
    use std::sync::Mutex;

    use objc2::MainThreadMarker;
    use objc2_app_kit::NSFontManager;
    use objc2_foundation::NSString;

    pub(crate) use super::FontPanelTarget as Target;

    static LAST_SEEN: Mutex<Option<(Target, String, i64)>> = Mutex::new(None);

    fn size_key(size: f64) -> i64 {
        (size * 100.0).round() as i64
    }

    /// Opens the system font panel pre-selected to `current_family` at
    /// `current_size`, for `target`. No-op off the main thread.
    pub fn open(target: Target, current_family: &str, current_size: f64) {
        let Some(mtm) = MainThreadMarker::new()
        else {
            return;
        };
        *LAST_SEEN.lock().unwrap() =
            Some((target, current_family.to_string(), size_key(current_size)));
        let manager = NSFontManager::sharedFontManager(mtm);
        if let Some(font) =
            objc2_app_kit::NSFont::fontWithName_size(&NSString::from_str(current_family),
                                                     current_size)
        {
            manager.setSelectedFont_isMultiple(&font, false);
        }
        if let Some(panel) = manager.fontPanel(true) {
            panel.makeKeyAndOrderFront(None);
        }
    }

    /// Returns the newly chosen `(target, family, size)` if it differs from
    /// the last value seen (by `open` or a prior poll). No-op off the main
    /// thread or before any target has opened the panel.
    pub fn poll_selection() -> Option<(Target, String, f64)> {
        let mtm = MainThreadMarker::new()?;
        let manager = NSFontManager::sharedFontManager(mtm);
        let selected = manager.selectedFont()?;
        let converted = manager.convertFont(&selected);
        let family = converted.familyName()?.to_string();
        let size = converted.pointSize();
        let mut last_seen = LAST_SEEN.lock().unwrap();
        let (target, _, _) = last_seen.clone()?;
        if *last_seen == Some((target, family.clone(), size_key(size))) {
            return None;
        }
        *last_seen = Some((target, family.clone(), size_key(size)));
        Some((target, family, size))
    }
}

/// Opens the OS emoji/character picker (the same panel as Edit > Emoji &
/// Symbols) so the user can pick an agent avatar from the full system
/// catalog instead of a curated list. AppKit inserts the chosen character
/// straight into whatever text field currently has keyboard focus, so the
/// caller must focus its input first - no callback or polling needed.
#[cfg(target_os = "macos")]
pub(crate) mod native_character_picker {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;

    /// No-op off the main thread.
    pub fn open() {
        let Some(mtm) = MainThreadMarker::new()
        else {
            return;
        };
        NSApplication::sharedApplication(mtm).orderFrontCharacterPalette(None);
    }
}

/// Embedded fonts (all SIL OFL licensed; see the matching `*-LICENSE.txt`
/// under `assets/fonts/`), so the app looks the same regardless of what's
/// installed on the system:
/// - Adamina: the app-wide default font (dialogs, buttons, settings labels, and
///   "title" text like agent names) - the renderer synthesizes bold for
///   `font_semibold`/`font_bold` text set in it.
/// - Manrope: applied explicitly, only to the workspace header and agent
///   sidebar cell text that isn't the agent's name (persona, status, folder,
///   git stats).
/// - JetBrains Mono: the default terminal font (`terminal_font_name`'s
///   default), a real monospace coding font rather than a mono variant of the
///   UI font, reliably resolvable regardless of what's installed on the system.
pub(crate) const ADAMINA_REGULAR: &[u8] = include_bytes!("../assets/fonts/Adamina-Regular.ttf");
pub(crate) const MANROPE_REGULAR: &[u8] = include_bytes!("../assets/fonts/Manrope-Regular.ttf");
pub(crate) const MANROPE_MEDIUM: &[u8] = include_bytes!("../assets/fonts/Manrope-Medium.ttf");
pub(crate) const MANROPE_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/Manrope-SemiBold.ttf");
pub(crate) const MANROPE_BOLD: &[u8] = include_bytes!("../assets/fonts/Manrope-Bold.ttf");
pub(crate) const JETBRAINS_MONO_REGULAR: &[u8] =
    include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf");
pub(crate) const JETBRAINS_MONO_BOLD: &[u8] =
    include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf");

pub(crate) const APP_ICON_PNG: &[u8] = include_bytes!("../assets/app-icon-32.png");

/// A small app-icon glyph for the leading edge of a custom `TitleBar`, sat
/// between the traffic lights and the title text.
pub(crate) fn app_titlebar_icon() -> impl IntoElement {
    let image = std::sync::Arc::new(gpui_kit::Image::from_bytes(gpui_kit::ImageFormat::Png,
                                                                APP_ICON_PNG.to_vec()));
    gpui_kit::img(image).w(px(16.))
                        .h(px(16.))
                        .rounded(px(4.))
                        .flex_shrink_0()
}

/// Replaces a `$HOME` prefix with `~` - matches the Swift reference's
/// `AgentTerminalView.shortenPath`.
pub(crate) fn shorten_path(path: &str) -> String {
    std::env::var("HOME").ok()
                         .and_then(|home| path.strip_prefix(&home).map(|rest| format!("~{rest}")))
                         .unwrap_or_else(|| path.to_string())
}

/// Registers the embedded font families and sets Adamina as the app-wide
/// default font (Manrope stays registered for the workspace header/cell
/// text that applies it explicitly), plus a distinct accent color, so the
/// app doesn't rely on the platform's generic UI font and neutral-gray
/// default theme.
pub(crate) fn apply_visual_identity(settings: &knot_core::Settings, cx: &mut App) {
    if let Err(error) =
        cx.text_system()
          .add_fonts(vec![std::borrow::Cow::Borrowed(ADAMINA_REGULAR),
                          std::borrow::Cow::Borrowed(MANROPE_REGULAR),
                          std::borrow::Cow::Borrowed(MANROPE_MEDIUM),
                          std::borrow::Cow::Borrowed(MANROPE_SEMIBOLD),
                          std::borrow::Cow::Borrowed(MANROPE_BOLD),
                          std::borrow::Cow::Borrowed(JETBRAINS_MONO_REGULAR),
                          std::borrow::Cow::Borrowed(JETBRAINS_MONO_BOLD),])
    {
        eprintln!("failed to register embedded fonts: {error}");
    }

    // The app-wide default stays the "title" font (Adamina) - Manrope
    // (`ui_font_name`) is applied explicitly only to the workspace header
    // and agent-cell text that isn't the agent's name, per the user's
    // request. Everything else (dialogs, buttons, settings labels) keeps
    // the existing font unchanged.
    let theme = cx.global_mut::<Theme>();
    theme.font_family = settings.title_font_name.clone().into();
    theme.mono_font_family = "JetBrains Mono".into();
    theme.font_size = px(settings.title_font_size as f32);
    let accent: gpui_kit::Hsla = rgb(0x3B82F6).into();
    let accent_hover: gpui_kit::Hsla = rgb(0x2563EB).into();
    let accent_active: gpui_kit::Hsla = rgb(0x1D4ED8).into();
    let white = gpui_kit::white();
    theme.colors.primary = accent;
    theme.colors.primary_hover = accent_hover;
    theme.colors.primary_active = accent_active;
    theme.colors.primary_foreground = white;
    theme.colors.button_primary = accent;
    theme.colors.button_primary_hover = accent_hover;
    theme.colors.button_primary_active = accent_active;
    theme.colors.button_primary_foreground = white;
    theme.colors.ring = accent;
    theme.colors.selection = accent.opacity(0.25);

    // `tokens` is a legacy snapshot of `colors` taken at construction time,
    // not re-derived on mutation (that's what Button/Switch actually read
    // for paint colors, e.g. `cx.theme().tokens.button_primary`).
    theme.tokens = ThemeTokens::from(&theme.colors);

    // Radius/scrollbar/typography reach rendering through a separately
    // mirrored Base layer that only `Theme::sync_base` re-derives.
    Theme::sync_base(cx);
}

/// The overlay layers `gpui_component::Root` does not draw for you.
///
/// `Root::render` paints its view plus the tooltip and menu overlays, but
/// *not* `active_dialogs` - a dialog opened with `open_alert_dialog` is
/// pushed onto the `Root` and then only appears if the application's own
/// root view renders this layer. Upstream says as much on
/// `render_dialog_layer`: "A dialog that opens into a root which never
/// renders this layer looks exactly like one that does not open." No window
/// in Knot rendered it, so every confirmation dialog in the app - restart
/// and remove agent, delete workspace, restore defaults, About - opened
/// invisibly and the click appeared to do nothing.
///
/// Every root view calls this, because `about_knot` opens its dialog on
/// whichever window happens to be active.
pub(crate) fn root_overlays(window: &mut Window, cx: &mut App) -> Vec<gpui_kit::AnyElement> {
    let mut layers = Vec::new();
    if let Some(layer) = Root::render_dialog_layer(window, cx) {
        layers.push(layer.into_any_element());
    }
    if let Some(layer) = Root::render_sheet_layer(window, cx) {
        layers.push(layer.into_any_element());
    }
    if let Some(layer) = Root::render_notification_layer(window, cx) {
        layers.push(layer.into_any_element());
    }
    layers
}

/// Names the running process, so macOS shows "Knot" in the application
/// menu rather than the executable's own lowercase name.
///
/// AppKit ignores the title given to the first `Menu` and labels the
/// application menu from the process name, which for an unbundled binary
/// is `argv[0]` - hence `knot`. `CFBundleName` would cover it once the
/// Rust app ships as a real `.app`; until then this sets the same thing at
/// runtime. Must run before the menu bar is built.
#[cfg(target_os = "macos")]
pub(crate) fn set_process_name(name: &str) {
    use objc2_foundation::{NSProcessInfo, NSString};

    let info = NSProcessInfo::processInfo();
    let name = NSString::from_str(name);
    // `setProcessName:` is declared on NSProcessInfo but not exposed by
    // objc2-foundation's generated bindings, so it goes through a raw
    // message send.
    unsafe {
        let _: () = objc2::msg_send![&*info, setProcessName: &*name];
    }
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn set_process_name(_name: &str) {}
