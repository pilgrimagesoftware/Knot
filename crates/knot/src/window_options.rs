use super::*;
/// The workspace window's title bar height. Its traffic lights are placed
/// from this, so the bar and the buttons cannot drift apart.
pub(crate) const WORKSPACE_TITLE_BAR_HEIGHT: f32 = 64.;

/// A macOS traffic light button, and the left inset the toolkit uses.
const TRAFFIC_LIGHT_DIAMETER: f32 = 12.;
const TRAFFIC_LIGHT_INSET: f32 = 9.;

pub(crate) fn manager_window_options(cx: &App) -> WindowOptions {
    WindowOptions { window_bounds: Some(WindowBounds::centered(size(px(800.), px(600.)), cx)),
                    window_min_size: Some(size(px(640.), px(420.))),
                    ..TitleBar::window_options() }
}

/// `saved` is the workspace's last known window frame, restored verbatim so
/// reopening puts the window back where the user left it; `None` (a
/// workspace never opened before) centres a default-sized window instead.
pub(crate) fn workspace_window_options(saved: Option<knot_core::SavedWindowBounds>, cx: &App)
                                       -> WindowOptions {
    let bounds = match saved {
        Some(saved) => WindowBounds::Windowed(gpui_kit::Bounds { origin:
                                                                     gpui_kit::point(px(saved.x),
                                                                                     px(saved.y)),
                                                                 size:   size(px(saved.width),
                                                                              px(saved.height)), }),
        None => WindowBounds::centered(size(px(960.), px(640.)), cx),
    };
    let mut options = WindowOptions { window_bounds: Some(bounds),
                                      window_min_size: Some(size(px(760.), px(520.))),
                                      ..TitleBar::window_options() };
    // AppKit places the traffic lights at a fixed offset, and the toolkit's
    // default (9px) centres them in its own ~30px bar. This window's bar is
    // taller, which left them stranded near the top edge and out of line
    // with the app icon and name beside them.
    if let Some(titlebar) = options.titlebar.as_mut() {
        let y = (WORKSPACE_TITLE_BAR_HEIGHT - TRAFFIC_LIGHT_DIAMETER) / 2.;
        titlebar.traffic_light_position = Some(gpui_kit::point(px(TRAFFIC_LIGHT_INSET), px(y)));
    }
    options
}

pub(crate) fn command_center_window_options(cx: &App) -> WindowOptions {
    WindowOptions { window_bounds: Some(WindowBounds::centered(size(px(960.), px(640.)), cx)),
                    window_min_size: Some(size(px(760.), px(520.))),
                    ..TitleBar::window_options() }
}

pub(crate) fn agent_window_options(title: &str, cx: &App) -> WindowOptions {
    WindowOptions { titlebar: Some(gpui_kit::TitlebarOptions { title: Some(title.to_string()
                                                                                .into()),
                                                               ..Default::default() }),
                    window_bounds: Some(WindowBounds::centered(size(px(520.), px(500.)), cx)),
                    window_min_size: Some(size(px(460.), px(460.))),
                    ..WindowOptions::default() }
}

/// The About window's fixed size. It is not resizable and not minimizable:
/// its content neither reflows usefully nor is worth keeping in the Dock, per
/// `openspec/specs/about-ui`.
///
/// The titlebar carries no text on macOS: the system's own About panel has
/// none, and the window already names the app in its body - two titles for
/// one window is what `knot-ui-conventions` rules out. Elsewhere a titled
/// window is the expectation, so the title is shown.
pub(crate) fn about_window_options(cx: &App) -> WindowOptions {
    let title = (!cfg!(target_os = "macos")).then(|| knot_core::l10n::t("about.title").into());
    WindowOptions { titlebar: Some(gpui_kit::TitlebarOptions { title,
                                                               ..Default::default() }),
                    window_bounds: Some(WindowBounds::centered(size(px(360.), px(560.)), cx)),
                    is_resizable: false,
                    is_minimizable: false,
                    ..WindowOptions::default() }
}

/// Fixed width for the settings window; only height varies per pane.
pub(crate) const SETTINGS_WINDOW_WIDTH: gpui_kit::Pixels = px(620.);

pub(crate) fn settings_window_options(cx: &App) -> WindowOptions {
    WindowOptions {
        titlebar: Some(gpui_kit::TitlebarOptions {
            title: Some("Settings".into()),
            ..Default::default()
        }),
        window_bounds: Some(WindowBounds::centered(
            size(
                SETTINGS_WINDOW_WIDTH,
                SettingsWindow::pane_target_height(SettingsTab::General),
            ),
            cx,
        )),
        window_min_size: Some(size(px(480.), px(320.))),
        ..WindowOptions::default()
    }
}
