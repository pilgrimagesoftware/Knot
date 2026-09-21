use gpui_kit::App;
use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::StyledExt;
use gpui_kit::base::h_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::group_box::GroupBox;
use gpui_kit::component::group_box::GroupBoxVariants;
use gpui_kit::div;
use gpui_kit::px;
use gpui_kit::rgb;

use super::font::font_label;
use crate::app_support::FontPanelTarget;
use crate::app_support::native_font_panel;
use crate::settings_window::SettingsWindow;

impl SettingsWindow {
    /// Right-aligned label column width shared by every settings row, so
    /// labels line up across a pane regardless of their length.
    const LABEL_WIDTH: f32 = 200.;

    /// A titled, bordered card grouping related controls. The title is
    /// deliberately larger than row content (`text_lg` vs. the default
    /// `text_base` used by row labels/controls) - a section header should
    /// never read smaller than what it's heading.
    pub(crate) fn group(title: &'static str) -> GroupBox {
        GroupBox::new().outline()
                       .title(div().text_lg().font_semibold().child(title))
    }

    /// A label + control row with the label right-aligned in a fixed-width
    /// column, matching the alignment convention already used by
    /// `AgentEditor`/`PersonaEditor`.
    pub(crate) fn row(label: impl Into<gpui_kit::SharedString>, control: impl IntoElement)
                      -> impl IntoElement {
        h_flex().gap_3()
                .items_center()
                .child(div().w(px(Self::LABEL_WIDTH))
                            .flex_shrink_0()
                            .text_right()
                            .child(label.into()))
                .child(control)
    }

    /// Like `row`, but baseline-aligned instead of center-aligned - for rows
    /// whose control is itself text (a read-only value, not a switch/button/
    /// input), so the value's text baseline lines up with the label's.
    pub(crate) fn text_row(label: impl Into<gpui_kit::SharedString>, control: impl IntoElement)
                           -> impl IntoElement {
        h_flex().gap_3()
                .items_baseline()
                .child(div().w(px(Self::LABEL_WIDTH))
                            .flex_shrink_0()
                            .text_right()
                            .child(label.into()))
                .child(control)
    }

    /// Muted description text lined up under a row's *control* column,
    /// not spanning the full card width - it explains the control above
    /// it, not the section as a whole.
    pub(crate) fn hint(cx: &Context<Self>, text: impl Into<gpui_kit::SharedString>)
                       -> impl IntoElement {
        h_flex().gap_3()
                .child(div().w(px(Self::LABEL_WIDTH)).flex_shrink_0())
                .child(div().flex_1()
                            .min_w_0()
                            .text_sm()
                            .whitespace_normal()
                            .text_color(cx.theme().muted_foreground)
                            .child(text.into()))
    }

    /// Renders `text` in the theme's monospace font, for values that are
    /// literally code/commands/identifiers (install commands, model names).
    pub(crate) fn mono_text(cx: &Context<Self>, text: impl Into<gpui_kit::SharedString>)
                            -> gpui_kit::Div {
        div().text_sm()
             .font_family(cx.theme().mono_font_family.clone())
             .child(text.into())
    }

    /// A small icon-only action button with a tooltip, used for utility
    /// actions (choose/clear/add/edit/delete/copy) instead of a text label -
    /// text buttons read as arbitrary activators, an icon reads as what it
    /// does. `danger` tints destructive actions (clear/delete) red.
    pub(crate) fn icon_button(id: impl Into<gpui_kit::ElementId>, icon_path: &'static str,
                              tooltip: impl Into<gpui_kit::SharedString>, danger: bool)
                              -> Button {
        let mut icon = Icon::default().path(icon_path);
        if danger {
            // `.ghost()` and `.danger()` are both button *variants* - only one
            // can apply, and ghost (no background) is what we want here - so
            // tint the icon itself red instead of switching variants.
            icon = icon.text_color(rgb(0xEF4444));
        }
        Button::new(id).icon(icon).tooltip(tooltip).ghost().small()
    }

    /// A single "Family, Npt" button that opens the OS font panel
    /// (`NSFontPanel`) pre-selected to the current font/size for `target` -
    /// one control picks both, since the panel itself has a size field.
    /// The choice comes back asynchronously via
    /// `native_font_panel::poll_selection`.
    ///
    /// The label is drawn in the family it names, so the row shows the face
    /// rather than only spelling it. That is what `cx` is for: the family has
    /// to be tested against the text system before it can be applied.
    #[cfg_attr(not(target_os = "macos"), allow(unused_variables))]
    pub(crate) fn font_picker_button(id: &'static str, target: FontPanelTarget, name: String,
                                     size: f64, cx: &App)
                                     -> Button {
        let preview = font_label(&name, size, &cx.text_system().all_font_names());
        let mut button = Button::new(id).label(preview.text);
        // Route taken for the preview (design.md's first trade-off): `Styled`
        // on `Button` refines the root div's style, and a div's text style
        // cascades to its descendants, so the family reaches the label
        // without replacing `.label()` with a styled child of our own. Only
        // the family is set - the label keeps the window's own text size
        // rather than `size`, so one row's setting cannot resize the section.
        if let Some(family) = preview.family {
            button = button.font_family(family);
        }
        button.on_click(move |_, _, _| {
                  #[cfg(target_os = "macos")]
                  native_font_panel::open(target, &name, size);
              })
    }
}
