//! The Bench tab: every bench entry, with edit (name and startup prompt)
//! and remove.
//!
//! Contract: `openspec/specs/settings-ui/spec.md`, "Bench tab". The workspace
//! window's bench popover lists the same entries for deploying; this tab is
//! where they are edited, which the popover has no room for.

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
use knot_core::{BenchAgent, Prompt, StartupPrompt};
use uuid::Uuid;

use super::super::bench_editor::open_bench_editor;
use crate::settings_window::SettingsWindow;

/// What a bench row says about its startup prompt: the library prompt's
/// name (or "Missing prompt"), "Custom", or nothing.
pub(crate) fn startup_summary(startup: Option<&StartupPrompt>, library: &[Prompt])
                              -> Option<String> {
    let (choice, _) = crate::startup_choice::initial_choice(startup);
    match choice {
        crate::startup_choice::StartupChoice::None => None,
        crate::startup_choice::StartupChoice::Custom => {
            Some(knot_core::l10n::t("settings.bench.custom"))
        }
        library_choice => Some(crate::startup_choice::choice_label(library_choice, library)),
    }
}

impl SettingsWindow {
    fn remove_bench_entry(&mut self, id: Uuid, cx: &mut Context<Self>) {
        if let Err(error) =
            crate::settings_global::write_persisting(cx, |settings| settings.remove_bench_agent(id))
        {
            eprintln!("failed to remove the bench entry: {error}");
        }
        cx.notify();
    }

    fn bench_row(&self, entry: BenchAgent, library: &[Prompt], index: usize,
                 cx: &mut Context<Self>)
                 -> impl IntoElement + use<> {
        let settings_window = cx.entity();
        let muted = cx.theme().muted_foreground;
        let folder = knot_core::folder_name(&entry.folder).unwrap_or_else(|| entry.folder.clone());
        let detail = match startup_summary(entry.startup_prompt.as_ref(), library) {
            Some(summary) => format!("{folder} · {summary}"),
            None => folder,
        };
        let edit = crate::controls::icon_button(("bench-edit", index),
                                                "icons/pencil.svg",
                                                knot_core::l10n::t("settings.bench.edit"),
                                                false).on_click({
                                                          let parent = settings_window.downgrade();
                                                          let entry = entry.clone();
                                                          move |_, _, app| {
                                                              open_bench_editor(parent.clone(),
                                                                                entry.clone(),
                                                                                app);
                                                          }
                                                      });
        let remove = crate::controls::icon_button(("bench-remove", index),
                                                  "icons/trash.svg",
                                                  knot_core::l10n::t("settings.bench.remove"),
                                                  true).on_click({
                         let settings_window = settings_window.clone();
                         let (id, name) = (entry.id, entry.name.clone());
                         move |_, window, app| {
                             let settings_window = settings_window.clone();
                             let body = knot_core::l10n::t_with("settings.bench.remove_body",
                                                                &[("name", &name)]);
                             window.open_alert_dialog(app, move |alert, _, _| {
                                 let settings_window = settings_window.clone();
                                 alert.title(knot_core::l10n::t("settings.bench.remove_title"))
                                      .description(body.clone())
                                      .confirm()
                                      .on_ok(move |_, _, app| {
                                          settings_window.update(app, |view, cx| {
                                                             view.remove_bench_entry(id, cx)
                                                         });
                                          true
                                      })
                             });
                         }
                     });
        h_flex().justify_between()
                .items_center()
                .gap_2()
                .child(div().text_lg().child(entry.avatar.clone()))
                .child(v_flex().flex_1()
                               .min_w_0()
                               .child(div().child(entry.name.clone()))
                               .child(div().text_sm().text_color(muted).child(detail)))
                .child(h_flex().flex_shrink_0().gap_1().child(edit).child(remove))
    }

    pub(crate) fn render_bench(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (bench, library) = {
            let settings = crate::settings_global::read(cx);
            (settings.bench_agents.clone(), settings.prompts.clone())
        };
        let list = if bench.is_empty() {
            div().text_sm()
                 .text_color(cx.theme().muted_foreground)
                 .child(knot_core::l10n::t("settings.bench.none_defined"))
                 .into_any_element()
        }
        else {
            v_flex().gap_3()
                    .children(bench.into_iter()
                                   .enumerate()
                                   .map(|(index, entry)| {
                                       self.bench_row(entry, &library, index, cx)
                                   }))
                    .into_any_element()
        };
        let list = div().id("bench-list")
                        .max_h(px(420.))
                        .overflow_y_scroll()
                        .child(list);
        v_flex().gap_3()
                .child(crate::controls::group(knot_core::l10n::t("settings.bench.bench")).child(list))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_summary_names_the_startup_prompts_form() {
        let library = vec![Prompt::new("gate", "make").unwrap()];
        assert_eq!(startup_summary(None, &library), None);
        assert_eq!(startup_summary(Some(&StartupPrompt::Library(library[0].id)), &library)
                       .as_deref(),
                   Some("gate"));
        assert_eq!(startup_summary(StartupPrompt::custom("x").as_ref(), &library),
                   Some(knot_core::l10n::t("settings.bench.custom")));
        assert_eq!(startup_summary(Some(&StartupPrompt::Library(Uuid::new_v4())), &library),
                   Some(knot_core::l10n::t("agent_editor.startup_prompt_missing")));
    }
}
