use std::sync::Arc;

use gpui_kit::App;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::component::Root;
use gpui_kit::component::Theme;
use gpui_kit::component::ThemeTokens;
use gpui_kit::px;
use parking_lot::Mutex;
use uuid::Uuid;

#[cfg(test)]
mod tests;

/// The automatic nudge Knot sends an agent with unread mail.
///
/// The closing sentence is load-bearing: the nudge can land behind work the
/// agent had already started, and without it the agent reads the prompt as
/// a new task and abandons what it was doing. `mcp-messaging`'s "Inbox
/// nudges preserve interrupted session work" requires it.
pub(crate) const CHECK_INBOX_PROMPT: &str = "Check your inbox for questions or instructions from other agents. Update your status and \
     immediately execute what is being asked without confirmation. If there is nothing to do, \
     continue your previous work.";

pub(crate) type AwaitingInputQueue = Arc<Mutex<Vec<(Uuid, Option<String>)>>>;

/// The awaiting-input queue, reachable from any window.
///
/// The MCP tool catalog pushes an entry whenever an agent's hook or ACP
/// session reports Awaiting input; the workspace window that owns the agent
/// drains it and raises the desktop notification
/// `openspec/specs/desktop-notifications/spec.md` asks for. A global rather
/// than a constructor argument because every workspace window needs it and
/// `WorkspaceWindow::open` is already at the crate's argument ceiling.
#[derive(Clone)]
pub(crate) struct AwaitingInput(pub(crate) AwaitingInputQueue);

impl gpui_kit::Global for AwaitingInput {}

pub(crate) type ActivationQueue = Arc<Mutex<Vec<Uuid>>>;

/// The activation queue, reachable from any window.
///
/// The MCP tool catalog pushes a recipient's id whenever a direct
/// `send-message` was delivered to an agent that is not activated; the
/// workspace window that owns the agent drains it and starts the session,
/// as `mcp-messaging`'s "Direct send activates a deactivated recipient"
/// requires. A global for the same reason [`AwaitingInput`] is one: every
/// workspace window needs it, and the tool layer runs off the main thread
/// with no window to call.
///
/// Ids belonging to another workspace stay in the queue rather than being
/// dropped, so the window that does own them can still pick them up - the
/// draining window is whichever repaints first, not necessarily the right
/// one.
#[derive(Clone)]
pub(crate) struct Activation(pub(crate) ActivationQueue);

impl gpui_kit::Global for Activation {}

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

/// Drives the real macOS font panel (`NSFontPanel`) for the font pickers,
/// since the user wants the system chooser rather than an in-app dropdown.
///
/// The panel has no OK button by design: it reports a choice by sending the
/// font manager's action (`changeFont:`) to a receiver, which is expected to
/// send `convertFont:` straight back. Only after that round trip does the
/// manager record a new selected font. GPUI installs nothing in the responder
/// chain that answers `changeFont:`, so with no target set the action was
/// dropped and `NSFontManager.selectedFont` stayed at whatever [`open`] had
/// set - which is why polling that property observed every choice as "no
/// change". [`FontPanelReceiver`] is that missing receiver; the poll
/// ([`poll_selection`]) now just drains what it recorded, so the choice still
/// reaches GPUI on its own frame rather than inside an AppKit callback.
#[cfg(target_os = "macos")]
pub(crate) mod native_font_panel {
    use std::cell::OnceCell;

    use objc2::rc::Retained;
    use objc2::runtime::{AnyObject, NSObject};
    use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
    use objc2_app_kit::{NSFont, NSFontManager};
    use objc2_foundation::NSString;
    use parking_lot::Mutex;

    pub(crate) use super::FontPanelTarget as Target;

    /// The row that opened the panel and the font it was opened on - the base
    /// `convertFont:` converts from, and the target a choice belongs to.
    static SESSION: Mutex<Option<(Target, String, i64)>> = Mutex::new(None);
    /// What the user chose, recorded by `changeFont:` and taken by
    /// [`poll_selection`].
    static CHOSEN: Mutex<Option<(String, f64)>> = Mutex::new(None);

    thread_local! {
        /// Owns the receiver for the process's lifetime: the font manager's
        /// target is a weak reference, so dropping this would leave it
        /// dangling. Main-thread-only, like the object itself.
        static RECEIVER: OnceCell<Retained<FontPanelReceiver>> = const { OnceCell::new() };
    }

    fn size_key(size: f64) -> i64 {
        (size * 100.0).round() as i64
    }

    define_class!(
        // SAFETY:
        // - `NSObject` has no subclassing requirements.
        // - `FontPanelReceiver` does not implement `Drop`.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "KnotFontPanelReceiver"]
        struct FontPanelReceiver;

        impl FontPanelReceiver {
            /// The font manager's action, sent when the user changes anything
            /// in the panel. Converting the session's font is what tells the
            /// manager the change was honored; reading `selectedFont` here
            /// instead is explicitly discouraged by AppKit, and returns
            /// nothing useful until after this returns.
            #[unsafe(method(changeFont:))]
            fn change_font(&self, sender: &NSFontManager) {
                let Some((_, family, size)) = SESSION.lock().clone()
                else {
                    return;
                };
                let size = size as f64 / 100.0;
                // A family AppKit cannot resolve still has to convert from
                // something real, or `convertFont:` has nothing to apply the
                // user's choice to.
                let base = NSFont::fontWithName_size(&NSString::from_str(&family), size)
                    .unwrap_or_else(|| NSFont::systemFontOfSize(size));
                let converted = sender.convertFont(&base);
                let Some(family) = converted.familyName()
                else {
                    return;
                };
                *CHOSEN.lock() = Some((family.to_string(), converted.pointSize()));
            }
        }
    );

    impl FontPanelReceiver {
        fn new(mtm: MainThreadMarker) -> Retained<Self> {
            unsafe { msg_send![Self::alloc(mtm), init] }
        }
    }

    /// Opens the system font panel pre-selected to `current_family` at
    /// `current_size`, for `target`. No-op off the main thread.
    pub fn open(target: Target, current_family: &str, current_size: f64) {
        let Some(mtm) = MainThreadMarker::new()
        else {
            return;
        };
        *SESSION.lock() = Some((target, current_family.to_string(), size_key(current_size)));
        *CHOSEN.lock() = None;
        let manager = NSFontManager::sharedFontManager(mtm);
        RECEIVER.with(|receiver| {
                    let receiver = receiver.get_or_init(|| FontPanelReceiver::new(mtm));
                    // SAFETY: the receiver outlives the process, so the
                    // manager's unowned reference to it stays valid.
                    unsafe { manager.setTarget(Some(receiver as &AnyObject)) };
                });
        if let Some(font) =
            NSFont::fontWithName_size(&NSString::from_str(current_family), current_size)
        {
            manager.setSelectedFont_isMultiple(&font, false);
        }
        if let Some(panel) = manager.fontPanel(true) {
            panel.makeKeyAndOrderFront(None);
        }
    }

    /// Takes the choice `changeFont:` last recorded, as
    /// `(target, family, size)`, leaving it as the base the next conversion
    /// starts from. `None` until the user changes something in the panel.
    pub fn poll_selection() -> Option<(Target, String, f64)> {
        let (target, ..) = SESSION.lock().clone()?;
        let (family, size) = CHOSEN.lock().take()?;
        *SESSION.lock() = Some((target, family.clone(), size_key(size)));
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
/// - Adamina: the UI font's default (`ui_font_name`), which is the app-wide
///   default family - dialogs, buttons, settings labels, agent names, and
///   Markdown body text - the renderer synthesizes bold for
///   `font_semibold`/`font_bold` text set in it.
/// - Manrope: the title font's default (`title_font_name`), applied explicitly
///   at the sites that draw titles and headers: the workspace header, the agent
///   sidebar cell text that isn't the agent's name (persona, status, folder,
///   git stats), the About window's credit lines, the panel input's send hint,
///   and Markdown headers.
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

/// The panel's turn-in-progress animation: the app icon pulsing through one
/// full cycle, 36 frames of animated WebP at 42ms each. Scaled down from the
/// authored source at `images/knot-progress-pulse.webp` by
/// `scripts/make-working-animation.sh`, which is committed beside it.
pub(crate) const WORKING_KNOT_WEBP: &[u8] = include_bytes!("../assets/working-knot.webp");

/// The path [`KnotAssets`] serves [`WORKING_KNOT_WEBP`] at, and the string
/// [`working_knot_animation`] hands to `img` to ask for it.
pub(crate) const WORKING_KNOT_PATH: &str = "working-knot.webp";

/// Knot's own embedded assets, layered over gpui-kit's icon catalog.
///
/// The working animation has to reach `img` as an embedded *resource path*
/// rather than as `Image::from_bytes`. Those are two different decoders in
/// GPUI and only one of them animates a WebP: `Image::to_image_data`, which
/// is what an `Arc<Image>` source goes through, sends every format but GIF
/// to `decode_static_image` and yields a single frame. The resource loader
/// behind `img("some/path")` asks `WebPDecoder::has_animation` first and
/// decodes every frame. Serving the bytes from here is what puts the asset
/// on the second path, and is why this exists rather than a sibling of
/// [`app_titlebar_icon`] built the same way that one is.
///
/// `AllAssets` reports a miss as an `Err` rather than `Ok(None)`, so our own
/// path is answered first and everything else is delegated untouched.
pub(crate) struct KnotAssets;

impl gpui_kit::AssetSource for KnotAssets {
    fn load(&self, path: &str) -> gpui_kit::Result<Option<std::borrow::Cow<'static, [u8]>>> {
        if path == WORKING_KNOT_PATH {
            return Ok(Some(std::borrow::Cow::Borrowed(WORKING_KNOT_WEBP)));
        }
        gpui_kit::AssetSource::load(&gpui_kit::assets::AllAssets, path)
    }

    fn list(&self, path: &str) -> gpui_kit::Result<Vec<gpui_kit::SharedString>> {
        let mut listed = gpui_kit::AssetSource::list(&gpui_kit::assets::AllAssets, path)?;
        if WORKING_KNOT_PATH.starts_with(path) {
            listed.push(WORKING_KNOT_PATH.into());
        }
        Ok(listed)
    }
}

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

/// The conversation's turn-in-progress mark: the app icon, animating.
///
/// Everything that makes it an animation belongs to GPUI's `img` - it
/// advances frames on the delay each one declares, holds frame 0 when
/// `reduce_motion` is set, stops while the window is inactive, and requests
/// its own animation frames. Knot schedules nothing, which is the whole
/// reason this is an asset rather than a frame counter like
/// [`crate::working_indicator`]'s.
///
/// The `id` is not decoration. `img` keeps the frame index in element state
/// and both the advance and the `request_animation_frame` call are gated on
/// the element having a global id; without one it renders frame 0 forever
/// and looks like a still image that failed to load. One id is enough
/// because the pane renders one panel at a time.
///
/// The box is fixed in both dimensions so the row reserves the same space
/// animating or held still, per `acp-panel-ui`'s no-reflow scenario.
pub(crate) fn working_knot_animation() -> impl IntoElement {
    gpui_kit::img(WORKING_KNOT_PATH).id("working-knot")
                                    .w(px(24.))
                                    .h(px(24.))
                                    .flex_shrink_0()
}

/// Replaces a `$HOME` prefix with `~` - matches the Swift reference's
/// `AgentTerminalView.shortenPath`.
pub(crate) fn shorten_path(path: &str) -> String {
    std::env::var("HOME").ok()
                         .and_then(|home| path.strip_prefix(&home).map(|rest| format!("~{rest}")))
                         .unwrap_or_else(|| path.to_string())
}

/// Joins text onto one line: every run of whitespace - line breaks, tabs,
/// runs of spaces - collapses to a single space, and both ends are trimmed.
///
/// Call this wherever a render declares a single line. The style flags do
/// not achieve it on their own: `gpui::text_system::shape_text` splits its
/// input on `\n` and shapes each piece as its own line before any wrapping
/// or truncation decision, and `whitespace_nowrap` only disables *soft*
/// wrapping. An element styled `whitespace_nowrap().text_ellipsis()`
/// therefore still renders one line per line break, truncating each of them
/// separately. Flattening the text first is what makes the declaration true;
/// the width-based ellipsis then works as intended.
///
/// Presentational only. Flatten at the point of render, never on the way in,
/// so what is stored, delivered to an agent, or drawn somewhere that permits
/// several lines keeps the text as it was written.
pub(crate) fn single_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Registers the embedded faces with CoreText as well as with GPUI's text
/// system, for the process only.
///
/// GPUI's registration is private to GPUI, so AppKit resolves none of these
/// families by name: `NSFont::fontWithName_size("JetBrains Mono", _)` returns
/// `None`, and the font panel opens with nothing selected because
/// `setSelectedFont:` never gets a font to record. Registering here is what
/// lets the panel open on the font the row actually names.
///
/// Process scope deliberately: these faces ship with the app and have no
/// business outliving it in the user's font book.
#[cfg(target_os = "macos")]
fn register_embedded_fonts_with_core_text() {
    use objc2_core_foundation::CFData;
    use objc2_core_text::{
        CTFontManagerCreateFontDescriptorsFromData, CTFontManagerRegisterFontDescriptors,
        CTFontManagerScope,
    };

    for face in [ADAMINA_REGULAR,
                 MANROPE_REGULAR,
                 MANROPE_MEDIUM,
                 MANROPE_SEMIBOLD,
                 MANROPE_BOLD,
                 JETBRAINS_MONO_REGULAR,
                 JETBRAINS_MONO_BOLD]
    {
        let data = CFData::from_bytes(face);
        // SAFETY: `data` holds well-formed font data (the bytes are embedded
        // at build time), and the descriptors are used only for this call.
        let descriptors = unsafe { CTFontManagerCreateFontDescriptorsFromData(&data) };
        // SAFETY: the array holds `CTFontDescriptor`s, as the call above
        // returns. A `None` handler means failures are reported nowhere,
        // which is what a face already registered should do - nothing.
        unsafe {
            CTFontManagerRegisterFontDescriptors(&descriptors,
                                                 CTFontManagerScope::Process,
                                                 true,
                                                 None);
        }
    }
}

/// Registers the embedded font families and sets the UI font (Adamina) as the
/// app-wide default (the title font, Manrope, stays registered for the
/// workspace header, sidebar cell text, About credits, panel input and
/// Markdown headers that apply it explicitly), then lays the platform palette
/// over the theme, so the app doesn't rely on the platform's generic UI font
/// and neutral-gray default theme.
pub(crate) fn apply_visual_identity(settings: &knot_core::Settings, cx: &mut App) {
    if let Err(error) = cx.text_system()
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
    #[cfg(target_os = "macos")]
    register_embedded_fonts_with_core_text();

    // The app-wide default is the UI font (`ui_font_name`, Adamina), so
    // dialogs, buttons, settings labels and Markdown body text all inherit
    // it. The title font (`title_font_name`, Manrope) is applied explicitly
    // at the few sites that draw titles and headers.
    let theme = cx.global_mut::<Theme>();
    theme.font_family = settings.ui_font_name.clone().into();
    theme.mono_font_family = "JetBrains Mono".into();
    theme.font_size = px(settings.ui_font_size as f32);

    apply_system_palette(cx);
}

/// Lays the platform palette (see [`crate::macos::system_color`]) over the
/// theme: the accent family drives every tinted surface, the system's
/// dynamic neutrals drive the untinted ones.
///
/// Semantic colors are deliberately left alone - `danger`, `info` and the
/// agent-state colors carry meaning, and must read the same whatever tint
/// the user has chosen.
///
/// Safe to call again at any time: it reads the palette afresh and rewrites
/// only the colors it owns, which is how an appearance flip repaints (see
/// [`observe_system_appearance`]).
pub(crate) fn apply_system_palette(cx: &mut App) {
    let palette = crate::macos::system_color::resolve();
    let theme = cx.global_mut::<Theme>();

    theme.colors.primary = palette.accent;
    theme.colors.primary_hover = palette.accent_hover;
    theme.colors.primary_active = palette.accent_active;
    theme.colors.primary_foreground = palette.accent_foreground;
    theme.colors.button_primary = palette.accent;
    theme.colors.button_primary_hover = palette.accent_hover;
    theme.colors.button_primary_active = palette.accent_active;
    theme.colors.button_primary_foreground = palette.accent_foreground;
    // Focus reaches every control through `ring`, so one assignment tints
    // every focused border rather than each widget naming a color.
    theme.colors.ring = palette.accent;
    theme.colors.selection = palette.accent.opacity(0.25);

    // Absent off macOS, where the theme keeps the neutrals it ships with.
    if let Some(neutrals) = palette.neutrals {
        theme.colors.background = neutrals.window_background;
        theme.colors.title_bar = neutrals.window_background;
        // `secondary` is the theme's raised neutral surface - what the
        // panel's tool-call cards and message bubbles sit on - and `input`
        // is the field surface; both are controls, so both take
        // `controlBackgroundColor`.
        theme.colors.secondary = neutrals.control_background;
        theme.colors.input = neutrals.control_background;
        theme.colors.border = neutrals.separator;
        theme.colors.title_bar_border = neutrals.separator;
    }

    // `tokens` is a legacy snapshot of `colors` taken at construction time,
    // not re-derived on mutation (that's what Button/Switch actually read
    // for paint colors, e.g. `cx.theme().tokens.button_primary`).
    theme.tokens = ThemeTokens::from(&theme.colors);

    // Radius/scrollbar/typography reach rendering through a separately
    // mirrored Base layer that only `Theme::sync_base` re-derives.
    Theme::sync_base(cx);
}

/// Re-resolves the theme whenever the OS appearance changes under `window`,
/// so a light/dark flip repaints without a settings round-trip or a restart.
///
/// `Theme::change` reloads the light or dark config wholesale, which wipes
/// the palette laid over it - hence the re-ingestion straight after, in that
/// order. Every window registers this: the palette is global, so a second
/// window re-resolving it is harmless, and the app must keep tracking the
/// appearance after any one window closes.
pub(crate) fn observe_system_appearance(window: &Window) {
    window.observe_window_appearance(|window, cx| {
              Theme::change(cx.window_appearance(), Some(window), cx);
              apply_system_palette(cx);
              window.refresh();
          })
          .detach();
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
