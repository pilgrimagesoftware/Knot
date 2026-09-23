use gpui_kit::App;
use gpui_kit::Bounds;
use gpui_kit::Pixels;
use gpui_kit::SharedString;
use gpui_kit::Size;
use gpui_kit::WindowBounds;
use gpui_kit::WindowOptions;
use gpui_kit::component::TitleBar;
use gpui_kit::px;
use gpui_kit::size;

use crate::consts;
use crate::settings_window::SettingsTab;
use crate::settings_window::SettingsWindow;

/// The workspace window's title bar height. Its traffic lights are placed
/// from this, so the bar and the buttons cannot drift apart.
pub(crate) const WORKSPACE_TITLE_BAR_HEIGHT: f32 = 64.;

/// A macOS traffic light button, and the left inset the toolkit uses.
const TRAFFIC_LIGHT_DIAMETER: f32 = 12.;
const TRAFFIC_LIGHT_INSET: f32 = 9.;

/// A window that draws its own header into the toolkit's title bar: the
/// three main windows, which put workspace controls up there.
///
/// Centred at `initial`, and no smaller than `min` however the user drags
/// it. Anything past that - a restored frame, traffic-light placement - the
/// caller sets on the options it gets back.
fn toolkit_bar_window(initial: Size<Pixels>, min: Size<Pixels>, cx: &App) -> WindowOptions {
    WindowOptions { window_bounds: Some(WindowBounds::centered(initial, cx)),
                    window_min_size: Some(min),
                    ..TitleBar::window_options() }
}

/// A window that keeps the OS title bar and names itself in it: the
/// dialogs, which per `knot-ui-conventions` carry their purpose there
/// rather than in a body heading.
///
/// `title` is optional because the About window deliberately has none on
/// macOS, and `min` because the same window is not resizable at all.
fn os_bar_window(title: Option<SharedString>, initial: Size<Pixels>, min: Option<Size<Pixels>>,
                 cx: &App)
                 -> WindowOptions {
    WindowOptions { titlebar: Some(gpui_kit::TitlebarOptions { title,
                                                               ..Default::default() }),
                    window_bounds: Some(WindowBounds::centered(initial, cx)),
                    window_min_size: min,
                    ..WindowOptions::default() }
}

pub(crate) fn manager_window_options(cx: &App) -> WindowOptions {
    toolkit_bar_window(size(px(800.), px(600.)), size(px(640.), px(420.)), cx)
}

/// Reconciles a remembered window frame with the displays actually attached,
/// reporting `None` when nothing sensible can be made of it and the caller
/// should fall back to its default placement.
///
/// The rules, in order (`openspec/specs/window-lifecycle`, "A restored window
/// opens where the user can reach it"):
///
/// - Bounds already inside an attached display are used unchanged.
/// - A size no attached display can hold falls back.
/// - Otherwise the frame is moved - never resized - by the smallest offset that
///   puts its top edge and a usable part of its width on the display it
///   overlaps most.
/// - A frame overlapping no display at all falls back.
///
/// Moving rather than resizing is deliberate: the size the user chose is
/// information, and a resize would also be written back by the bounds observer
/// and so become permanent.
pub(crate) fn reconcile_bounds(saved: Bounds<Pixels>, displays: &[Bounds<Pixels>])
                               -> Option<Bounds<Pixels>> {
    if displays.is_empty() {
        return None;
    }
    if displays.iter()
               .any(|display| contains_rect(display, &saved))
    {
        return Some(saved);
    }
    if !displays.iter().any(|display| {
                           saved.size.width <= display.size.width
                           && saved.size.height <= display.size.height
                       })
    {
        return None;
    }
    let display =
        displays.iter()
                .max_by(|a, b| overlap_area(&saved, a).total_cmp(&overlap_area(&saved, b)))?;
    if overlap_area(&saved, display) <= 0. {
        return None;
    }
    Some(nudge_onto(saved, display))
}

/// Whether `outer` wholly contains `inner`.
fn contains_rect(outer: &Bounds<Pixels>, inner: &Bounds<Pixels>) -> bool {
    inner.origin.x >= outer.origin.x
    && inner.origin.y >= outer.origin.y
    && inner.origin.x + inner.size.width <= outer.origin.x + outer.size.width
    && inner.origin.y + inner.size.height <= outer.origin.y + outer.size.height
}

/// How much of `frame` lies on `display`, as an area - what picks the display
/// a frame straddling two of them is pulled onto.
fn overlap_area(frame: &Bounds<Pixels>, display: &Bounds<Pixels>) -> f32 {
    let overlap = frame.intersect(display);
    if overlap.is_empty() {
        return 0.;
    }
    f32::from(overlap.size.width) * f32::from(overlap.size.height)
}

/// Moves `frame` the least it can so its grab strip is on `display`.
///
/// Each axis is clamped independently to the nearest legal origin, which is
/// what makes the move minimal.
fn nudge_onto(frame: Bounds<Pixels>, display: &Bounds<Pixels>) -> Bounds<Pixels> {
    let left = f32::from(display.origin.x);
    let top = f32::from(display.origin.y);
    let right = left + f32::from(display.size.width);
    let bottom = top + f32::from(display.size.height);
    let width = f32::from(frame.size.width);

    // At least `WINDOW_MIN_VISIBLE_WIDTH` of the window has to be on screen at
    // each edge, so it can neither hide off the right nor off the left.
    let visible = consts::WINDOW_MIN_VISIBLE_WIDTH.min(width);
    let x = f32::from(frame.origin.x).clamp(left - (width - visible), right - visible);
    // The whole title bar, not a sliver of it - and never above the top edge,
    // where macOS puts the menu bar.
    let strip = consts::WINDOW_GRAB_STRIP_HEIGHT.min(f32::from(frame.size.height));
    let y = f32::from(frame.origin.y).clamp(top, (bottom - strip).max(top));

    Bounds { origin: gpui_kit::point(px(x), px(y)),
             size:   frame.size, }
}

/// Where a workspace window should actually open, given what was remembered
/// for it and the displays attached now.
///
/// `None` - nothing remembered, or nothing sensible to be made of it - leaves
/// the centred default placement.
///
/// The caller keeps this to tell its own placement apart from a move the user
/// made: the remembered frame must survive a reopen on a smaller display
/// (`openspec/specs/window-lifecycle`, "Reconciliation is not written back"),
/// so what this returns must never be persisted as though the user chose it.
pub(crate) fn reconciled_workspace_bounds(saved: Option<knot_core::SavedWindowBounds>, cx: &App)
                                          -> Option<Bounds<Pixels>> {
    let saved = saved?;
    let frame = Bounds { origin: gpui_kit::point(px(saved.x), px(saved.y)),
                         size:   size(px(saved.width), px(saved.height)), };
    let displays = cx.displays()
                     .iter()
                     .map(|display| display.visible_bounds())
                     .collect::<Vec<_>>();
    reconcile_bounds(frame, &displays)
}

/// `placed` is where the window should open, from
/// [`reconciled_workspace_bounds`]; `None` centres a default-sized window.
pub(crate) fn workspace_window_options(placed: Option<Bounds<Pixels>>, cx: &App) -> WindowOptions {
    let mut options = toolkit_bar_window(size(px(960.), px(640.)), size(px(760.), px(520.)), cx);
    if let Some(placed) = placed {
        options.window_bounds = Some(WindowBounds::Windowed(placed));
    }
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
    toolkit_bar_window(size(px(960.), px(640.)), size(px(760.), px(520.)), cx)
}

pub(crate) fn agent_window_options(title: &str, cx: &App) -> WindowOptions {
    os_bar_window(Some(title.to_string().into()),
                  size(px(520.), px(500.)),
                  Some(size(px(460.), px(460.))),
                  cx)
}

/// The broadcast sheet: a utility dialog, so its purpose goes in the OS
/// titlebar rather than an in-body heading (`knot-ui-conventions`). Shorter
/// than the agent editor - it holds one field and two buttons.
pub(crate) fn broadcast_window_options(cx: &App) -> WindowOptions {
    let title = knot_core::l10n::t("broadcast.window_title");
    os_bar_window(Some(title.into()),
                  size(px(480.), px(280.)),
                  Some(size(px(360.), px(220.))),
                  cx)
}

/// The commit window's size. Taller than the broadcast sheet: a commit
/// message is a subject and a body, and a field too short to show both
/// encourages the one-line messages the hint argues against.
pub(crate) fn commit_window_options(cx: &App) -> WindowOptions {
    let title = knot_core::l10n::t("git_panel.commit_title");
    os_bar_window(Some(title.into()),
                  size(px(520.), px(340.)),
                  Some(size(px(380.), px(260.))),
                  cx)
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
    WindowOptions { is_resizable: false,
                    is_minimizable: false,
                    ..os_bar_window(title, size(px(360.), px(560.)), None, cx) }
}

/// The Import window's size. Resizable, unlike About: both lists scroll
/// inside fixed-height regions, so a taller window shows more of them at
/// once, which is the whole reason to drag it.
///
/// It carries a title on every platform, including macOS: unlike the About
/// box, the window's body does not name itself, and an untitled window is
/// also left out of the macOS Window menu.
pub(crate) fn import_window_options(cx: &App) -> WindowOptions {
    WindowOptions { titlebar: Some(gpui_kit::TitlebarOptions {
                        title: Some(knot_core::l10n::t("import.title").into()),
                        ..Default::default()
                    }),
                    window_bounds: Some(WindowBounds::centered(size(px(520.), px(600.)), cx)),
                    window_min_size: Some(size(px(420.), px(360.))),
                    ..WindowOptions::default() }
}

/// Fixed width for the settings window; only height varies per pane.
pub(crate) const SETTINGS_WINDOW_WIDTH: Pixels = px(620.);

pub(crate) fn settings_window_options(cx: &App) -> WindowOptions {
    let height = SettingsWindow::pane_target_height(SettingsTab::General);
    os_bar_window(Some(knot_core::l10n::t("settings.title").into()),
                  size(SETTINGS_WINDOW_WIDTH, height),
                  Some(size(px(480.), px(320.))),
                  cx)
}
