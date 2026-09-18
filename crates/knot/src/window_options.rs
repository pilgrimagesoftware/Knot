use super::*;
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
    WindowOptions { window_bounds: Some(bounds),
                    window_min_size: Some(size(px(760.), px(520.))),
                    ..TitleBar::window_options() }
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
