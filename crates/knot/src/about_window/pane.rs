//! What the About window draws: the icon, the version and build a bug report
//! needs, and the credits.

use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::base::StyledExt;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::Button;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::div;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::px;

use super::build_info::build_details;
use super::build_info::build_identifier;
use super::build_info::version;
use super::window::AboutWindow;

/// The icon shown at the top of the window. `APP_ICON_PNG` is the 32px
/// title-bar glyph, too small to read as the app's identity here, so this
/// embeds the full-resolution packaging icon and renders it down.
const ABOUT_ICON_PNG: &[u8] = include_bytes!("../../assets/icon/icon.png");

/// The rendered icon's edge length. The macOS About box shows the app icon
/// at its 128pt size.
const ICON_SIZE: f32 = 128.;

impl AboutWindow {
    /// One credit line. Muted and small: the credits are the least of what
    /// someone opens this window to read, and must not compete with the
    /// version. Centred per line, not only as a block - a line that wraps
    /// would otherwise sit left-aligned inside a centred column.
    fn credit(cx: &Context<Self>, key: &str) -> gpui_kit::Div {
        div().text_xs()
             .text_center()
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
                // Every line but the app name is set in the title font; the
                // name keeps the app-wide UI font, as the workspace header
                // and agent rows do.
                .font_family(self.title_font.clone())
                .child(gpui_kit::img(icon).w(px(ICON_SIZE)).h(px(ICON_SIZE)))
                .child(div().text_2xl()
                            .font_semibold()
                            .font_family(cx.theme().font_family.clone())
                            .child(knot_core::l10n::t("app.name")))
                // The version and build are the text a bug report needs, so
                // the text itself copies them - a copy button beside it was
                // one more thing to aim at, and the toolkit's selectable
                // text is an input control, which would read as an editable
                // field in an About box.
                .child(v_flex().id("about-build-details")
                               .items_center()
                               .px_3()
                               .py_1()
                               .rounded(px(6.))
                               .cursor_pointer()
                               .hover(|style| style.bg(cx.theme().muted))
                               .tooltip({
                                   let hint = knot_core::l10n::t("about.copy_details");
                                   move |window, cx| Tooltip::new(hint.clone()).build(window, cx)
                               })
                               .on_click(move |_, _, app| {
                                   app.write_to_clipboard(gpui_kit::ClipboardItem::new_string(details.clone()));
                               })
                               .child(div().text_sm()
                                           .text_center()
                                           .text_color(cx.theme().muted_foreground)
                                           .child(format!("{} {}",
                                                          knot_core::l10n::t("about.version_label"),
                                                          version())))
                               .child(div().text_sm()
                                           .text_center()
                                           .text_color(cx.theme().muted_foreground)
                                           .child(format!("{} {}",
                                                          knot_core::l10n::t("about.build_label"),
                                                          build_identifier()))))
                .child(div().mt_2()
                            .text_xs()
                            .text_center()
                            .text_color(cx.theme().muted_foreground)
                            .child(knot_core::l10n::t("about.copyright")))
                // The Rust app is its own work; Skwad is what it was
                // derived from, and saying so is owed to Kochava Studios.
                .child(div().text_xs()
                            .text_center()
                            .text_color(cx.theme().muted_foreground)
                            .child(knot_core::l10n::t("about.derived_from")))
                .child(v_flex().mt_4()
                               .gap_1()
                               .items_center()
                               .child(Self::credit(cx, "about.credits.author"))
                               .child(Self::credit(cx, "about.credits.license"))
                               .child(div().mt_2()
                                           .text_xs()
                                           .text_center()
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
