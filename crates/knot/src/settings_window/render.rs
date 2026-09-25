//! The frame the panes sit in: the tab strip, the scroll region around the
//! selected pane, and how tall the window is made for it.

use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::tab::Tab;
use gpui_kit::component::tab::TabBar;
use gpui_kit::div;
use gpui_kit::px;
use gpui_kit::size;

use super::tab::SettingsTab;
use super::window::SettingsWindow;
use crate::window_options::SETTINGS_WINDOW_WIDTH;

impl SettingsWindow {
    /// Target window height for each pane's content, capped so a long
    /// Personas list can't push the window arbitrarily tall - it scrolls
    /// within the cap instead (see `render` below).
    pub(crate) fn pane_target_height(tab: SettingsTab) -> gpui_kit::Pixels {
        match tab {
            SettingsTab::General => px(560.),
            SettingsTab::Coding => px(440.),
            SettingsTab::Personas => px(600.),
            SettingsTab::Prompts => px(600.),
            SettingsTab::Bench => px(560.),
            SettingsTab::Autopilot => px(660.),
            SettingsTab::Voice => px(520.),
            SettingsTab::Mcp => px(600.),
            SettingsTab::Terminal => px(380.),
            SettingsTab::Keyboard => px(560.),
        }
    }

    fn render_tab_strip(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_window = cx.entity();
        let selected_index = SettingsTab::ALL.iter()
                                             .position(|tab| *tab == self.selected_tab)
                                             .unwrap_or(0);
        TabBar::new("settings-tabs").underline()
                                    .selected_index(selected_index)
                                    .children(SettingsTab::ALL.map(|tab| {
                                                                  Tab::new().label(tab.label())
                                                              }))
                                    .on_click(move |index, window, app| {
                                        let tab = SettingsTab::ALL[*index];
                                        settings_window.update(app, |view, cx| {
                                                           view.selected_tab = tab;
                                                           cx.notify();
                                                       });
                                        window.resize(size(SETTINGS_WINDOW_WIDTH,
                                                           Self::pane_target_height(tab)));
                                    })
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let body = match self.selected_tab {
            SettingsTab::General => self.render_general(cx).into_any_element(),
            SettingsTab::Coding => self.render_coding(cx).into_any_element(),
            SettingsTab::Personas => self.render_personas(cx).into_any_element(),
            SettingsTab::Prompts => self.render_prompts(cx).into_any_element(),
            SettingsTab::Bench => self.render_bench(cx).into_any_element(),
            SettingsTab::Autopilot => self.render_autopilot(cx).into_any_element(),
            SettingsTab::Voice => self.render_voice(cx).into_any_element(),
            SettingsTab::Mcp => self.render_mcp(cx).into_any_element(),
            SettingsTab::Terminal => self.render_appearance(cx).into_any_element(),
            SettingsTab::Keyboard => self.render_keyboard(cx).into_any_element(),
        };

        // The list tabs and Keyboard manage their own scroll region (only the
        // list scrolls; the title, blurb and action rows stay pinned) -
        // scrolling the body too would let both containers move at once and
        // make the group's title/border appear to drift.
        let mut settings_body = div().id("settings-body").flex_1().min_h_0();
        settings_body = if matches!(self.selected_tab,
                                    SettingsTab::Personas
                                    | SettingsTab::Prompts
                                    | SettingsTab::Bench
                                    | SettingsTab::Keyboard)
        {
            settings_body.overflow_hidden()
        }
        else {
            settings_body.overflow_y_scroll()
        };

        v_flex().size_full()
                .gap_3()
                .p_4()
                .bg(cx.theme().background)
                .child(self.render_tab_strip(cx))
                .child(settings_body.child(body))
                .children(crate::app_support::root_overlays(window, cx))
    }
}
