//! The About window - what it shows and how it opens
//! (`openspec/specs/about-ui`).
//!
//! It is a window rather than a dialog: an About box that blocks the window
//! it opened over is not what any desktop platform does, and the modal alert
//! this replaces could name neither the version nor the build. A single
//! instance, like the settings window - choosing About again raises the one
//! that is open.

use super::*;

/// The icon shown at the top of the window. `APP_ICON_PNG` is the 32px
/// title-bar glyph, too small to read as the app's identity here, so this
/// embeds the full-resolution packaging icon and renders it down.
const ABOUT_ICON_PNG: &[u8] = include_bytes!("../../assets/icon/icon.png");

/// The rendered icon's edge length. The macOS About box shows the app icon
/// at its 128pt size.
const ICON_SIZE: f32 = 128.;

/// Stamped by `build.rs` in place of the commit when there is no repository
/// to ask - a source tarball, or a machine without `git`. Must match the
/// `UNKNOWN` constant there.
const UNKNOWN_COMMIT: &str = "unknown";

/// The commit and build date `build.rs` stamped into this binary.
const BUILD_COMMIT: &str = env!("KNOT_BUILD_COMMIT");
const BUILD_DATE: &str = env!("KNOT_BUILD_DATE");

/// The running binary's released version - the `knot` crate's version, which
/// is what the release workflow bumps, so the window cannot go stale against
/// it.
pub(crate) fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The build identifier: the date the binary was built, and the commit it
/// was built from. Two builds of one version made from different commits
/// differ here, which is the whole point of showing it.
pub(crate) fn build_identifier() -> String {
    format_build(BUILD_DATE, BUILD_COMMIT)
}

/// Says the commit is unknown rather than leaving a blank where an
/// identifier belongs: a user reading "Build 2026-09-21" cannot tell whether
/// the commit was omitted or the window is broken.
fn format_build(date: &str, commit: &str) -> String {
    if commit == UNKNOWN_COMMIT {
        format!("{date}, {}", knot_core::l10n::t("about.commit_unknown"))
    }
    else {
        format!("{date}, {commit}")
    }
}

/// What the copy action puts on the clipboard - everything a bug report
/// needs about which binary was running, in one line.
pub(crate) fn build_details() -> String {
    format!("{} {} ({})",
            knot_core::l10n::t("app.name"),
            version(),
            build_identifier())
}

/// Registers the `AboutKnot` handler over the window handle it keeps.
///
/// The handle lives in this closure rather than in `run`, so nothing outside
/// can open a second About window past the single-instance check. `run` and
/// the window tests both register through here, so the tests exercise the
/// wiring the app actually installs.
pub(crate) fn register_about_action(cx: &mut App) {
    let handle: Rc<RefCell<Option<AnyWindowHandle>>> = Rc::new(RefCell::new(None));
    cx.on_action(move |_: &crate::app_bootstrap::AboutKnot, cx| open_about_window(&handle, cx));
}

/// Raises the open About window, or opens one.
///
/// Whether the last window is still open is asked by updating it: a closed
/// window fails its update, which is the same test the settings window uses
/// and is why no close observer is needed to clear the handle.
///
/// Unlike the alert dialog this replaces, no `cx.defer` is needed. A menu
/// dispatch runs inside the active window's update, so opening a dialog *on
/// that window* was re-entrant; opening a new window is not.
pub(crate) fn open_about_window(handle: &Rc<RefCell<Option<AnyWindowHandle>>>, cx: &mut App) {
    if let Some(existing) = *handle.borrow()
       && existing.update(cx, |_, window, _| window.activate_window())
                  .is_ok()
    {
        return;
    }
    match cx.open_window(about_window_options(cx), |window, cx| {
                let view = cx.new(AboutWindow::new);
                cx.new(|cx| Root::new(view, window, cx).bg(cx.theme().background))
            }) {
        Ok(window) => *handle.borrow_mut() = Some(window.into()),
        Err(error) => eprintln!("failed to open the About window: {error}"),
    }
}

pub(crate) struct AboutWindow {
    /// Focused on first render so the window has a key target for `Escape`.
    /// A window with nothing focused never sees the key event at all.
    focus: gpui_kit::FocusHandle,
}

impl AboutWindow {
    fn new(cx: &mut Context<Self>) -> Self {
        Self { focus: cx.focus_handle(), }
    }

    /// One credit line. Muted and small: the credits are the least of what
    /// someone opens this window to read, and must not compete with the
    /// version.
    fn credit(cx: &Context<Self>, key: &str) -> gpui_kit::Div {
        div().text_xs()
             .text_color(cx.theme().muted_foreground)
             .child(knot_core::l10n::t(key))
    }
}

impl Render for AboutWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Focusing here rather than at open time: the focus handle only has
        // an element to attach to once this tree exists.
        if window.focused(cx).is_none() {
            window.focus(&self.focus.clone(), cx);
        }

        let icon = std::sync::Arc::new(gpui_kit::Image::from_bytes(gpui_kit::ImageFormat::Png,
                                                                   ABOUT_ICON_PNG.to_vec()));
        let details = build_details();

        v_flex().track_focus(&self.focus)
                .on_key_down(cx.listener(|_, event: &gpui_kit::KeyDownEvent, window, _| {
                    // Scoped to this window's own focus handle rather than an
                    // app-wide `Escape` binding, which would take the key
                    // away from every other window.
                    if event.keystroke.key == "escape" {
                        window.remove_window();
                    }
                }))
                .size_full()
                .items_center()
                .gap_2()
                .px_8()
                .py_8()
                .bg(cx.theme().background)
                .child(gpui_kit::img(icon).w(px(ICON_SIZE)).h(px(ICON_SIZE)))
                .child(div().text_2xl()
                            .font_semibold()
                            .child(knot_core::l10n::t("app.name")))
                .child(h_flex().gap_2()
                               .items_center()
                               .child(v_flex().items_center()
                                              .child(div().text_sm()
                                                          .text_color(cx.theme().muted_foreground)
                                                          .child(format!("{} {}",
                                                                 knot_core::l10n::t("about.version_label"),
                                                                 version())))
                                              .child(div().text_sm()
                                                          .text_color(cx.theme().muted_foreground)
                                                          .child(format!("{} {}",
                                                                 knot_core::l10n::t("about.build_label"),
                                                                 build_identifier()))))
                               // A copy action rather than selectable text:
                               // the toolkit's selectable text is an input
                               // control, which would read as an editable
                               // field in an About box.
                               .child(SettingsWindow::icon_button("about-copy-build",
                                                                  "icons/copy.svg",
                                                                  knot_core::l10n::t("about.copy_details"),
                                                                  false).on_click(move |_, _, app| {
                                          app.write_to_clipboard(ClipboardItem::new_string(details.clone()));
                                      })))
                .child(div().mt_2()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(knot_core::l10n::t("about.copyright")))
                .child(v_flex().mt_4()
                               .gap_1()
                               .items_center()
                               .child(Self::credit(cx, "about.credits.author"))
                               .child(Self::credit(cx, "about.credits.license"))
                               .child(div().mt_2()
                                           .text_xs()
                                           .font_semibold()
                                           .text_color(cx.theme().muted_foreground)
                                           .child(knot_core::l10n::t("about.credits.built_with")))
                               .child(Self::credit(cx, "about.credits.toolkit"))
                               .child(Self::credit(cx, "about.credits.terminal"))
                               .child(Self::credit(cx, "about.credits.fonts")))
                // A macOS About box has no button: it is closed from its
                // window chrome. Elsewhere that is not the expectation, so
                // the window carries an explicit Close.
                .when(!cfg!(target_os = "macos"), |column| {
                    column.child(div().mt_4()
                                      .child(Button::new("about-close").label(knot_core::l10n::t("about.close"))
                                                                       .on_click(|_, window: &mut Window, _| {
                                                                           window.remove_window()
                                                                       })))
                })
                .children(crate::app_support::root_overlays(window, cx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_is_the_crates_own() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
        assert!(version().split('.').count() >= 3,
                "the version is not a released version number: {}",
                version());
    }

    #[test]
    fn a_known_commit_is_shown_with_the_date() {
        assert_eq!(format_build("2026-09-21", "1a2b3c4d5e6f"),
                   "2026-09-21, 1a2b3c4d5e6f");
    }

    #[test]
    fn an_unknown_commit_says_so_rather_than_leaving_a_blank() {
        let build = format_build("2026-09-21", UNKNOWN_COMMIT);
        assert!(build.starts_with("2026-09-21, "),
                "the date went missing: {build}");
        assert!(build.contains(&knot_core::l10n::t("about.commit_unknown")),
                "an unstamped commit did not say it was unknown: {build}");
        assert!(!build.ends_with(&format!(", {UNKNOWN_COMMIT}")),
                "the raw stamp is shown where a commit belongs: {build}");
    }

    #[test]
    fn the_build_identifier_names_this_build() {
        let build = build_identifier();
        assert!(build.starts_with(BUILD_DATE),
                "the build is not dated: {build}");
        assert!(build.len() > BUILD_DATE.len(),
                "the build names no commit at all: {build}");
    }

    #[test]
    fn the_copied_details_name_the_app_version_and_build() {
        let details = build_details();
        assert!(details.starts_with(&knot_core::l10n::t("app.name")));
        assert!(details.contains(version()),
                "the version is missing: {details}");
        assert!(details.contains(&build_identifier()),
                "the build is missing: {details}");
    }
}
