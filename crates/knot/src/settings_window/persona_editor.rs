use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::Styled;
use gpui_kit::WeakEntity;
use gpui_kit::Window;
use gpui_kit::WindowBounds;
use gpui_kit::WindowOptions;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Root;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::input::Input;
use gpui_kit::component::input::InputState;
use gpui_kit::component::input::Textarea;
use gpui_kit::component::input::TextareaState;
use gpui_kit::div;
use gpui_kit::px;
use gpui_kit::size;
use uuid::Uuid;

use crate::app_support::observe_system_appearance;
use crate::settings_window::SettingsWindow;

pub(crate) fn persona_editor_window_options(title: &'static str, cx: &App) -> WindowOptions {
    WindowOptions { titlebar: Some(gpui_kit::TitlebarOptions { title: Some(title.into()),
                                                               ..Default::default() }),
                    window_bounds: Some(WindowBounds::centered(size(px(460.), px(380.)), cx)),
                    window_min_size: Some(size(px(400.), px(320.))),
                    ..WindowOptions::default() }
}

/// Opens the persona add/edit window. `persona` is `None` for "Add Persona…"
/// and `Some` (fields pre-filled) for a row's edit button.
pub(crate) fn open_persona_editor(parent: WeakEntity<SettingsWindow>,
                                  persona: Option<knot_core::Persona>, cx: &mut App) {
    let editing_id = persona.as_ref().map(|p| p.id);
    let title = if editing_id.is_some() {
        "Edit Persona"
    }
    else {
        "New Persona"
    };
    let name = persona.as_ref().map(|p| p.name.clone()).unwrap_or_default();
    let instructions = persona.as_ref()
                              .map(|p| p.instructions.clone())
                              .unwrap_or_default();
    let options = persona_editor_window_options(title, cx);
    let _ = cx.open_window(options, move |window, cx| {
                  // Every window tracks the OS appearance, so a light/dark flip
                  // re-resolves the system palette and repaints.
                  observe_system_appearance(window);
                  let name_input = cx.new(|cx| {
                                         InputState::new(window, cx).placeholder("Persona name")
                                                                    .default_value(name)
                                     });
                  name_input.update(cx, |state, cx| state.focus(window, cx));
                  let instructions_input = cx.new(|cx| {
                                                 TextareaState::new(window, cx)
                .placeholder("Instructions")
                .default_value(instructions)
                                             });
                  let view = cx.new(|_| PersonaEditor { parent,
                                                        editing_id,
                                                        name_input,
                                                        instructions_input,
                                                        error: None });
                  cx.new(|cx| Root::new(view, window, cx))
              });
}

pub(crate) struct PersonaEditor {
    parent:             WeakEntity<SettingsWindow>,
    editing_id:         Option<Uuid>,
    name_input:         Entity<InputState>,
    instructions_input: Entity<TextareaState>,
    error:              Option<String>,
}

impl PersonaEditor {
    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let name = self.name_input.read(cx).value().trim().to_string();
        if name.is_empty() {
            self.error = Some("Enter a persona name.".to_string());
            cx.notify();
            return;
        }
        let instructions = self.instructions_input.read(cx).value().trim().to_string();
        let Some(parent) = self.parent.upgrade()
        else {
            window.remove_window();
            return;
        };
        parent.update(cx, |view, view_cx| {
                  let result = match self.editing_id {
                      Some(id) => view.settings.update_persona(id, name, instructions),
                      None => view.settings.add_persona(name, instructions).map(|_| ()),
                  };
                  if let Err(error) = result {
                      eprintln!("failed to save persona: {error}");
                  }
                  view_cx.notify();
              });
        window.remove_window();
    }
}

impl Render for PersonaEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_3()
            .p_5()
            .bg(cx.theme().background)
            .child(
                h_flex()
                    .gap_2()
                    .child(div().w(px(100.)).text_right().child("Name"))
                    .child(Input::new(&self.name_input).flex_1()),
            )
            .child(
                h_flex()
                    .flex_1()
                    .gap_2()
                    .child(div().w(px(100.)).text_right().child("Instructions"))
                    .child(
                        Textarea::new(&self.instructions_input)
                            .flex_1()
                            .h_full()
                            .font_family(cx.theme().mono_font_family.clone()),
                    ),
            )
            .children(
                self.error
                    .as_ref()
                    .map(|error| div().text_sm().child(error.clone())),
            )
            .child(
                h_flex()
                    .flex_shrink_0()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("cancel-persona-editor")
                            .label("Cancel")
                            .on_click(|_, window, _| window.remove_window()),
                    )
                    .child(
                        Button::new("save-persona-editor")
                            .label("Save")
                            .primary()
                            .on_click(cx.listener(|editor, _, window, cx| editor.save(window, cx))),
                    ),
            )
            .children(crate::app_support::root_overlays(window, cx))
    }
}
