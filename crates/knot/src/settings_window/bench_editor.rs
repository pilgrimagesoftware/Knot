//! The bench-entry editor the Bench tab opens: a name and a startup prompt,
//! the two fields an entry does not take from the agent it was saved from.
//!
//! Contract: `openspec/specs/settings-ui/spec.md`, "Bench tab".

use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::Styled;
use gpui_kit::Subscription;
use gpui_kit::WeakEntity;
use gpui_kit::Window;
use gpui_kit::base::Disableable;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Root;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::Input;
use gpui_kit::component::input::InputEvent;
use gpui_kit::component::input::InputState;
use gpui_kit::component::input::Textarea;
use gpui_kit::component::input::TextareaState;
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::div;
use gpui_kit::px;
use uuid::Uuid;

use crate::app_support::observe_system_appearance;
use crate::settings_window::SettingsWindow;
use crate::settings_window::persona_editor::persona_editor_window_options;
use crate::startup_choice::{self, StartupChoice};

/// Opens the editor for bench entry `entry`.
pub(crate) fn open_bench_editor(parent: WeakEntity<SettingsWindow>,
                                entry: knot_core::BenchAgent, cx: &mut App) {
    let options =
        persona_editor_window_options(knot_core::l10n::t("settings.bench_editor.title"), cx);
    let (choice, custom_text) = startup_choice::initial_choice(entry.startup_prompt.as_ref());
    let _ = cx.open_window(options, move |window, cx| {
                  observe_system_appearance(window);
                  let name_input =
                      cx.new(|cx| InputState::new(window, cx).default_value(entry.name.clone()));
                  name_input.update(cx, |state, cx| state.focus(window, cx));
                  let custom_input =
                      cx.new(|cx| TextareaState::new(window, cx).default_value(custom_text));
                  let view = cx.new(|cx| {
                                   let subscriptions =
                                       vec![cx.subscribe(&name_input, repaint_on_change),
                                            cx.subscribe(&custom_input, repaint_on_change)];
                                   BenchEditor { parent,
                                                 id: entry.id,
                                                 name_input,
                                                 choice,
                                                 custom_input,
                                                 _subscriptions: subscriptions }
                               });
                  cx.new(|cx| Root::new(view, window, cx))
              });
}

/// The name gates the save button and the custom text drives the warning,
/// so each edit re-renders.
fn repaint_on_change<E>(_: &mut BenchEditor, _: Entity<E>, event: &InputEvent,
                        cx: &mut Context<BenchEditor>) {
    if matches!(event, InputEvent::Change) {
        cx.notify();
    }
}

pub(crate) struct BenchEditor {
    parent:         WeakEntity<SettingsWindow>,
    id:             Uuid,
    name_input:     Entity<InputState>,
    choice:         StartupChoice,
    custom_input:   Entity<TextareaState>,
    _subscriptions: Vec<Subscription>,
}

impl BenchEditor {
    fn name(&self, cx: &App) -> String {
        self.name_input.read(cx).value().trim().to_string()
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.name(cx);
        if name.is_empty() {
            return;
        }
        let custom = self.custom_input.read(cx).value().to_string();
        let startup_prompt = startup_choice::submitted(self.choice, &custom);
        let id = self.id;
        if let Err(error) = crate::settings_global::write_persisting(cx, |settings| {
            settings.update_bench_agent(id, name, startup_prompt)
        }) {
            eprintln!("failed to save the bench entry: {error}");
        }
        if let Some(parent) = self.parent.upgrade() {
            parent.update(cx, |_, cx| cx.notify());
        }
        window.remove_window();
    }
}

impl Render for BenchEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let library = crate::settings_global::read(cx).prompts.clone();
        let editor = cx.entity();
        let picker = startup_choice::picker("bench-editor-startup-prompt",
                                            self.choice,
                                            library,
                                            move |choice, app| {
                                                editor.update(app, |e, cx| {
                                                          e.choice = choice;
                                                          cx.notify();
                                                      })
                                            });
        let custom = (self.choice == StartupChoice::Custom).then(|| {
                         let text = self.custom_input.read(cx).value().to_string();
                         let warning = crate::prompt_text::unknown_variables_warning(&text);
                         let target = self.custom_input.clone();
                         let insert = Button::new("bench-editor-variables")
                .label(knot_core::l10n::t("prompt_variables.insert"))
                .dropdown_caret(true)
                .dropdown_menu(move |menu, _, _| crate::prompt_text::variable_menu(menu, &target));
                         v_flex().flex_1()
                                 .gap_1()
                                 .child(Textarea::new(&self.custom_input).flex_1().h_full())
                                 .child(div().child(insert))
                                 .children(warning.map(|warning| {
                                                      div().text_xs()
                                                           .text_color(cx.theme().warning)
                                                           .child(warning)
                                                  }))
                     });
        let label = |key: &str| {
            div().w(px(120.))
                 .text_right()
                 .child(knot_core::l10n::t(key))
        };
        v_flex()
            .size_full()
            .gap_3()
            .p_5()
            .bg(cx.theme().background)
            .child(h_flex().gap_2()
                           .child(label("settings.bench_editor.name"))
                           .child(Input::new(&self.name_input).flex_1()))
            .child(h_flex().gap_2()
                           .child(label("settings.bench_editor.startup_prompt"))
                           .child(picker))
            .children(custom)
            .child(div().flex_1())
            .child(
                h_flex()
                    .flex_shrink_0()
                    .justify_end()
                    .gap_2()
                    .child(Button::new("cancel-bench-editor")
                               .label(knot_core::l10n::t("settings.bench_editor.cancel"))
                               .on_click(|_, window, _| window.remove_window()))
                    .child(Button::new("save-bench-editor")
                               .label(knot_core::l10n::t("settings.bench_editor.save"))
                               .primary()
                               .disabled(self.name(cx).is_empty())
                               .on_click(cx.listener(|editor, _, window, cx| editor.save(window, cx)))),
            )
            .children(crate::app_support::root_overlays(window, cx))
    }
}
