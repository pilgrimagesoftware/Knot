//! What the Import window draws: one section per source Knot can bring
//! definitions in from.
//!
//! Contract: `openspec/specs/import-ui/spec.md`, over
//! `openspec/specs/data-import/spec.md`.
//!
//! Both sections follow the same shape: what the source holds, a checkbox per
//! record, and a button that imports only what is ticked. Nothing is written
//! on the strength of opening the window. A source with nothing in it says so
//! in place of its list rather than being hidden - a hidden section is
//! indistinguishable from one that does not exist, and the user cannot tell
//! whether Knot looked.
//!
//! Every function here reads state and returns elements. What changes that
//! state lives in [`super::window`].

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
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::div;
use gpui_kit::px;
use knot_core::import::ImportResult;

use super::window::ImportWindow;
use crate::controls::{group, icon_button};

impl ImportWindow {
    pub(super) fn render_import(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_3()
                .child(self.render_refresh_row(cx))
                .child(self.render_personas_section(cx))
                .child(self.render_skwad_section(cx))
                .children(self.render_result(cx))
    }

    /// Re-scan every source. The window scans once when it opens, so a
    /// definition written while it is open would otherwise not be offered
    /// until the window was reopened.
    fn render_refresh_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let import_window = cx.entity();
        h_flex().justify_end().child(icon_button("import-refresh",
                                                 "icons/rotate-ccw.svg",
                                                 knot_core::l10n::t("import.refresh"),
                                                 false).on_click(move |_, _, app| {
                                                           import_window.update(app, |view, cx| {
                                                                            view.refresh();
                                                                            cx.notify();
                                                                        });
                                                       }))
    }

    /// Personas from a coding agent's subagent definitions.
    fn render_personas_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let import_window = cx.entity();
        let definitions = self.sources.subagents.definitions.as_slice();
        let selected = self.subagent_selection.len();

        let body = if definitions.is_empty() {
            Self::empty_state("import.personas_none", cx).into_any_element()
        }
        else {
            v_flex().gap_2()
                    .children(definitions.iter().enumerate().map(|(index, definition)| {
                                  let source = definition.source.clone();
                                  let checked = self.subagent_selection.contains(&source);
                                  let import_window = import_window.clone();
                                  Checkbox::new(("import-definition", index)).label(definition.name
                                                                                             .clone())
                          .checked(checked)
                          .on_click(move |checked, _, app| {
                              let source = source.clone();
                              let checked = *checked;
                              import_window.update(app, |view, cx| {
                                               if checked {
                                                   view.subagent_selection.insert(source);
                                               }
                                               else {
                                                   view.subagent_selection.remove(&source);
                                               }
                                               cx.notify();
                                           });
                          })
                              }))
                    .into_any_element()
        };

        group(knot_core::l10n::t("import.personas_title"))
            .child(Self::list("import-personas-list", body))
            .child(Self::action_row(
                "import-personas",
                selected,
                definitions.is_empty(),
                cx,
                move |view, cx| view.import_selected_definitions(cx),
            ))
    }

    /// Workspaces from a Skwad installation.
    fn render_skwad_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let import_window = cx.entity();
        let workspaces = self.sources.skwad.workspaces.as_slice();
        let selected = self.workspace_selection.len();

        let body = if workspaces.is_empty() {
            Self::empty_state("import.workspaces_none", cx).into_any_element()
        }
        else {
            v_flex().gap_2()
                    .children(workspaces.iter().enumerate().map(|(index, workspace)| {
                                  let id = workspace.id;
                                  let checked = self.workspace_selection.contains(&id);
                                  let import_window = import_window.clone();
                                  Checkbox::new(("import-workspace", index))
                        .label(Self::workspace_label(workspace))
                        .checked(checked)
                        .on_click(move |checked, _, app| {
                            let checked = *checked;
                            import_window.update(app, |view, cx| {
                                             if checked {
                                                 view.workspace_selection.insert(id);
                                             }
                                             else {
                                                 view.workspace_selection.remove(&id);
                                             }
                                             cx.notify();
                                         });
                        })
                              }))
                    .into_any_element()
        };

        group(knot_core::l10n::t("import.workspaces_title"))
            .child(Self::list("import-skwad-list", body))
            .child(Self::action_row(
                "import-skwad",
                selected,
                workspaces.is_empty(),
                cx,
                move |view, cx| view.import_selected_workspaces(cx),
            ))
    }

    /// A workspace's row label: its name and how many agents come with it, so
    /// the choice is made on what it actually brings across.
    pub(crate) fn workspace_label(workspace: &knot_core::Workspace) -> String {
        let agents = knot_core::l10n::pluralize(workspace.agent_ids.len() as u64,
                                                "count.agent",
                                                "count.agents");
        format!("{} ({agents})", workspace.name)
    }

    /// What a section shows in place of its list when its source holds
    /// nothing.
    fn empty_state(key: &str, cx: &Context<Self>) -> impl IntoElement {
        div().text_sm()
             .text_color(cx.theme().muted_foreground)
             .child(knot_core::l10n::t(key))
    }

    /// A section's scrollable list region, bounded so a long list scrolls in
    /// place rather than pushing the section below it off the window.
    fn list(id: impl Into<gpui_kit::ElementId>, body: gpui_kit::AnyElement) -> impl IntoElement {
        div().id(id).max_h(px(200.)).overflow_y_scroll().child(body)
    }

    /// The Import button for one section, disabled until something is ticked
    /// so the control says what it needs rather than doing nothing when
    /// pressed.
    fn action_row(id: &'static str, selected: usize, source_empty: bool, cx: &mut Context<Self>,
                  run: impl Fn(&mut Self, &mut Context<Self>) + 'static)
                  -> impl IntoElement {
        let import_window = cx.entity();
        let label = if selected == 0 {
            knot_core::l10n::t("import.import_selected")
        }
        else {
            knot_core::l10n::t_with("import.import_count", &[("count", &selected.to_string())])
        };

        h_flex().justify_end().child(Button::new(id).label(label)
                                                    .primary()
                                                    .small()
                                                    .disabled(selected == 0 || source_empty)
                                                    .on_click(move |_, _, app| {
                                                        import_window.update(app, |view, cx| {
                                                                         run(view, cx)
                                                                     });
                                                    }))
    }

    /// The summary of the last import, shown only after one has run.
    fn render_result(&self, cx: &Context<Self>) -> Option<impl IntoElement> {
        let result: &ImportResult = self.result.as_ref()?;
        let lines = result.summary_lines();
        let body = if lines.is_empty() {
            vec![knot_core::l10n::t("import.nothing_to_do")]
        }
        else {
            lines
        };

        Some(group(knot_core::l10n::t("import.result_title"))
            .child(v_flex().gap_1().children(body.into_iter().map(|line| {
                div().text_sm().whitespace_normal().text_color(cx.theme().muted_foreground).child(line)
            }))))
    }
}
