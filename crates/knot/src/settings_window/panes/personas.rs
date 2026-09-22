use std::collections::BTreeMap;

use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::base::Disableable;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::WindowExt;
use gpui_kit::div;
use gpui_kit::px;
use uuid::Uuid;

use super::super::persona_editor::open_persona_editor;
use crate::settings_window::SettingsWindow;

impl SettingsWindow {
    /// Truncates `instructions` to `max_chars`, appending an ellipsis when
    /// truncated so a persona list row stays a single line.
    pub(crate) fn persona_preview(instructions: &str, max_chars: usize) -> String {
        let truncated: String = instructions.chars().take(max_chars).collect();
        if instructions.chars().count() > max_chars {
            format!("{truncated}…")
        }
        else {
            truncated
        }
    }

    /// How many agents each persona is assigned to. A persona still in
    /// use can't be deleted: the agents referencing it would keep a
    /// `persona_id` pointing at nothing, which reads as "no persona"
    /// everywhere without ever saying the instructions were dropped.
    pub(crate) fn personas_in_use(agents: &[knot_agents::Agent]) -> BTreeMap<Uuid, usize> {
        let mut counts = BTreeMap::new();
        for persona in agents.iter().filter_map(|agent| agent.persona_id) {
            *counts.entry(persona).or_insert(0) += 1;
        }
        counts
    }

    /// `personas_in_use` against the live store - the window's own
    /// `Settings` snapshot is from the moment it opened and misses agents
    /// created since.
    fn live_personas_in_use(&self) -> BTreeMap<Uuid, usize> {
        Self::personas_in_use(self.store.lock().agents())
    }

    /// The delete button's tooltip, which doubles as the reason it is
    /// disabled when the persona is assigned to agents.
    pub(crate) fn persona_delete_tooltip(in_use: usize) -> String {
        match in_use {
            0 => "Delete persona".to_string(),
            1 => "In use by 1 agent".to_string(),
            count => format!("In use by {count} agents"),
        }
    }

    fn delete_persona(&mut self, id: Uuid, cx: &mut Context<Self>) {
        // Checked again here, not just on the button: the dialog that
        // gets us here is opened from a rendered row, and an agent could
        // have taken the persona in between.
        if self.live_personas_in_use().get(&id).copied().unwrap_or(0) > 0 {
            cx.notify();
            return;
        }
        if let Err(error) = self.settings.remove_persona(id) {
            eprintln!("failed to remove persona: {error}");
        }
        cx.notify();
    }

    fn restore_default_personas(&mut self, cx: &mut Context<Self>) {
        if let Err(error) = self.settings.restore_default_personas() {
            eprintln!("failed to restore default personas: {error}");
        }
        cx.notify();
    }

    pub(crate) fn render_personas(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_window = cx.entity();
        let personas: Vec<knot_core::Persona> = self.settings
                                                    .active_personas()
                                                    .into_iter()
                                                    .cloned()
                                                    .collect();

        let in_use = self.live_personas_in_use();

        let list = if personas.is_empty() {
            div().text_sm()
                 .text_color(cx.theme().muted_foreground)
                 .child(knot_core::l10n::t("settings.personas.none_defined"))
                 .into_any_element()
        }
        else {
            v_flex()
                    .gap_3()
                    .children(personas.into_iter().enumerate().map(|(index, persona)| {
                        let id = persona.id;
                        let preview = Self::persona_preview(&persona.instructions, 80);
                        let in_use = in_use.get(&id).copied().unwrap_or(0);
                        h_flex()
                            .justify_between()
                            .items_center()
                            .gap_2()
                            .child(
                                v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .child(div().child(persona.name.clone()))
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(preview),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .flex_shrink_0()
                                    .gap_1()
                                    .child(
                                        Self::icon_button(
                                            ("persona-edit", index),
                                            "icons/pencil.svg",
                                            "Edit persona",
                                            false,
                                        )
                                        .on_click({
                                            let parent = settings_window.downgrade();
                                            let persona = persona.clone();
                                            move |_, _, app| {
                                                open_persona_editor(
                                                    parent.clone(),
                                                    Some(persona.clone()),
                                                    app,
                                                );
                                            }
                                        }),
                                    )
                                    .child(
                                        // Disabled rather than hidden while
                                        // agents still reference it, with the
                                        // count as the tooltip so the button
                                        // says why it won't work.
                                        Self::icon_button(
                                            ("persona-delete", index),
                                            "icons/trash.svg",
                                            Self::persona_delete_tooltip(in_use),
                                            true,
                                        )
                                        .disabled(in_use > 0)
                                        .on_click({
                                            let settings_window = settings_window.clone();
                                            let name = persona.name.clone();
                                            move |_, window, app| {
                                                let settings_window = settings_window.clone();
                                                window.open_alert_dialog(app, {
                                                    let name = name.clone();
                                                    move |alert, _, _| {
                                                        let settings_window =
                                                            settings_window.clone();
                                                        alert
                                                        .title(knot_core::l10n::t("settings.personas.delete_persona"))
                                                        .description(format!(
                                                            "This permanently deletes \"{name}\". \
                                                             This can't be undone."
                                                        ))
                                                        .confirm()
                                                        .on_ok(move |_, _, app| {
                                                            settings_window.update(app, |view, cx| {
                                                                view.delete_persona(id, cx);
                                                            });
                                                            true
                                                        })
                                                    }
                                                });
                                            }
                                        }),
                                    ),
                            )
                    }))
                    .into_any_element()
        };
        // Bounded so the list scrolls in place instead of pushing the group's
        // title/action row (which must stay visible) off the top of the
        // window - same cap philosophy as the window's own per-pane height.
        let list = div().id("personas-list")
                        .max_h(px(420.))
                        .overflow_y_scroll()
                        .child(list);

        v_flex().gap_3().child(
            Self::group(knot_core::l10n::t("settings.personas.personas"))
                .child(
                    h_flex()
                        .justify_between()
                        .child(
                            Self::icon_button(
                                "personas-add",
                                "icons/plus.svg",
                                "Add Persona…",
                                false,
                            )
                            .on_click({
                                let parent = settings_window.downgrade();
                                move |_, _, app| {
                                    open_persona_editor(parent.clone(), None, app);
                                }
                            }),
                        )
                        .child(
                            Self::icon_button(
                                "personas-restore-defaults",
                                "icons/rotate-ccw.svg",
                                "Restore Defaults",
                                false,
                            )
                            .on_click({
                                let settings_window = settings_window.clone();
                                move |_, window, app| {
                                    let settings_window = settings_window.clone();
                                    window.open_alert_dialog(app, move |alert, _, _| {
                                        let settings_window = settings_window.clone();
                                        alert
                                            .title(knot_core::l10n::t("settings.personas.restore_defaults"))
                                            .description(
                                                knot_core::l10n::t("settings.personas.restore_defaults_body"),
                                            )
                                            .confirm()
                                            .on_ok(move |_, _, app| {
                                                settings_window.update(app, |view, cx| {
                                                    view.restore_default_personas(cx);
                                                });
                                                true
                                            })
                                    });
                                }
                            }),
                        ),
                )
                .child(list),
        )
    }
}
