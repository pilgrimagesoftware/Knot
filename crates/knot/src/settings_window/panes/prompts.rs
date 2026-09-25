//! The Prompts tab: the prompt library, with add, edit and delete.
//!
//! Contract: `openspec/specs/settings-ui/spec.md`, "Prompts tab".
//!
//! Deleting a prompt nothing uses happens at once; deleting one that agents
//! or bench entries name as their startup prompt asks first and says how
//! many, since each will launch without one afterwards. Agents are counted
//! from the live store - this window's settings snapshot misses agents
//! created since it opened - and bench entries from the live surface.

use gpui_kit::App;
use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::WindowExt;
use gpui_kit::div;
use gpui_kit::px;
use knot_core::{Prompt, PromptReferences, StartupPrompt};
use uuid::Uuid;

use super::super::prompt_editor::open_prompt_editor;
use crate::settings_window::SettingsWindow;

/// A prompt's text as one line for its row: line breaks become spaces, and
/// the result is truncated like a persona preview.
pub(crate) fn prompt_preview(text: &str, max_chars: usize) -> String {
    let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    SettingsWindow::persona_preview(&one_line, max_chars)
}

/// How many `agents` and `bench` entries name prompt `id` as their startup
/// prompt.
pub(crate) fn prompt_references(id: Uuid, agents: &[knot_agents::Agent],
                                bench: &[knot_core::BenchAgent])
                                -> PromptReferences {
    let names = |startup: &Option<StartupPrompt>| matches!(startup, Some(StartupPrompt::Library(target)) if *target == id);
    PromptReferences { agents: agents.iter()
                                     .filter(|agent| names(&agent.startup_prompt))
                                     .count(),
                       bench:  bench.iter()
                                    .filter(|entry| names(&entry.startup_prompt))
                                    .count(), }
}

impl SettingsWindow {
    fn live_prompt_references(&self, id: Uuid, cx: &App) -> PromptReferences {
        let bench = crate::settings_global::read(cx).bench_agents.clone();
        prompt_references(id, self.store.lock().agents(), &bench)
    }

    fn delete_prompt(&mut self, id: Uuid, cx: &mut Context<Self>) {
        if let Err(error) =
            crate::settings_global::write_persisting(cx, |settings| settings.remove_prompt(id))
        {
            eprintln!("failed to delete the prompt: {error}");
        }
        cx.notify();
    }

    fn prompt_row(&self, prompt: Prompt, index: usize, cx: &mut Context<Self>)
                  -> impl IntoElement + use<> {
        let settings_window = cx.entity();
        let preview = prompt_preview(&prompt.text, 80);
        let edit = crate::controls::icon_button(("prompt-edit", index),
                                                "icons/pencil.svg",
                                                knot_core::l10n::t("settings.prompts.edit"),
                                                false).on_click({
                       let parent = settings_window.downgrade();
                       let prompt = prompt.clone();
                       move |_, _, app| {
                           open_prompt_editor(parent.clone(), Some(prompt.clone()), app);
                       }
                   });
        let delete = crate::controls::icon_button(("prompt-delete", index),
                                                  "icons/trash.svg",
                                                  knot_core::l10n::t("settings.prompts.delete"),
                                                  true).on_click({
                                                           let settings_window =
                                                               settings_window.clone();
                                                           let prompt = prompt.clone();
                                                           move |_, window, app| {
                                                               confirm_delete(&settings_window,
                                                                              &prompt,
                                                                              window,
                                                                              app);
                                                           }
                                                       });
        h_flex().justify_between()
                .items_center()
                .gap_2()
                .child(v_flex().flex_1()
                               .min_w_0()
                               .child(div().child(prompt.name.clone()))
                               .child(div().text_sm()
                                           .text_color(cx.theme().muted_foreground)
                                           .child(preview)))
                .child(h_flex().flex_shrink_0().gap_1().child(edit).child(delete))
    }

    pub(crate) fn render_prompts(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_window = cx.entity();
        let prompts = crate::settings_global::read(cx).prompts.clone();
        let list = if prompts.is_empty() {
            div().text_sm()
                 .text_color(cx.theme().muted_foreground)
                 .child(knot_core::l10n::t("settings.prompts.none_defined"))
                 .into_any_element()
        }
        else {
            v_flex().gap_3()
                    .children(prompts.into_iter()
                                     .enumerate()
                                     .map(|(index, prompt)| self.prompt_row(prompt, index, cx)))
                    .into_any_element()
        };
        // Only the list scrolls, as on the Personas tab: the group's title
        // and its add action stay in place.
        let list = div().id("prompts-list")
                        .max_h(px(420.))
                        .overflow_y_scroll()
                        .child(list);
        v_flex().gap_3().child(
            crate::controls::group(knot_core::l10n::t("settings.prompts.prompts"))
                .child(
                    h_flex().child(
                        crate::controls::icon_button("prompts-add",
                                                     "icons/plus.svg",
                                                     knot_core::l10n::t("settings.prompts.add"),
                                                     false)
                            .on_click({
                                let parent = settings_window.downgrade();
                                move |_, _, app| open_prompt_editor(parent.clone(), None, app)
                            }),
                    ),
                )
                .child(list),
        )
    }
}

/// Delete `prompt` at once when nothing uses it; otherwise confirm first,
/// saying how many agents and bench entries will lose their startup prompt.
fn confirm_delete(settings_window: &gpui_kit::Entity<SettingsWindow>, prompt: &Prompt,
                  window: &mut gpui_kit::Window, app: &mut App) {
    let id = prompt.id;
    let references = settings_window.read(app).live_prompt_references(id, app);
    if references.is_empty() {
        settings_window.update(app, |view, cx| view.delete_prompt(id, cx));
        return;
    }
    let body = knot_core::l10n::t_with("settings.prompts.delete_body",
                                       &[("name", &prompt.name),
                                         ("agents",
                                          &knot_core::l10n::pluralize(references.agents as u64,
                                                                      "count.agent",
                                                                      "count.agents")),
                                         ("bench",
                                          &knot_core::l10n::pluralize(references.bench as u64,
                                                                      "count.bench_entry",
                                                                      "count.bench_entries"))]);
    let settings_window = settings_window.clone();
    window.open_alert_dialog(app, move |alert, _, _| {
              let settings_window = settings_window.clone();
              alert.title(knot_core::l10n::t("settings.prompts.delete_title"))
                   .description(body.clone())
                   .confirm()
                   .on_ok(move |_, _, app| {
                       settings_window.update(app, |view, cx| view.delete_prompt(id, cx));
                       true
                   })
          });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_preview_is_one_line() {
        assert_eq!(prompt_preview("run\nthe  gate", 80), "run the gate");
        assert_eq!(prompt_preview(&"x".repeat(100), 10),
                   format!("{}…", "x".repeat(10)));
    }

    #[test]
    fn references_count_agents_and_bench_entries_naming_the_prompt() {
        let id = Uuid::new_v4();
        let mut store = knot_agents::AgentStore::new();
        store.create("/a",
                     knot_agents::CreateOptions { startup_prompt:
                                                      Some(StartupPrompt::Library(id)),
                                                  ..Default::default() });
        store.create("/b", knot_agents::CreateOptions::default());
        let mut entry = knot_core::BenchAgent::new(Uuid::new_v4(), "e", None, "/e");
        entry.startup_prompt = Some(StartupPrompt::Library(id));

        let references = prompt_references(id, store.agents(), &[entry]);

        assert_eq!(references,
                   PromptReferences { agents: 1,
                                      bench:  1, });
        assert!(prompt_references(Uuid::new_v4(), store.agents(), &[]).is_empty());
    }
}
