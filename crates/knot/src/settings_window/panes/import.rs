//! The Import tab: one section per source Knot can bring definitions in from.
//!
//! Contract: `openspec/specs/settings-ui/spec.md` - "Import tab", over
//! `openspec/specs/data-import/spec.md`.
//!
//! Both sections follow the same shape: what the source holds, a checkbox per
//! record, and a button that imports only what is ticked. Nothing is written
//! on the strength of opening the tab. A source with nothing in it says so in
//! place of its list rather than being hidden - a hidden section is
//! indistinguishable from one that does not exist, and the user cannot tell
//! whether Knot looked.
//!
//! Scanning is I/O, so it happens once when the tab is entered and is cached
//! in [`ImportSources`]; the render path only reads that cache.

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
use knot_core::import::{ImportResult, SkwadSource, SubagentScan, SubagentTool};
use uuid::Uuid;

use crate::settings_window::SettingsWindow;

/// What the Import tab found when it last looked.
///
/// Held rather than re-read per frame: GPUI re-renders on every keystroke, and
/// two directory walks plus a plist parse per frame is exactly the I/O the
/// render path must not do.
#[derive(Default)]
pub(crate) struct ImportSources {
    pub(crate) subagents: SubagentScan,
    pub(crate) skwad:     SkwadSource,
}

impl SettingsWindow {
    /// Scan every source, unless a scan is already cached.
    ///
    /// Called when the Import tab is entered, never from `render`.
    pub(crate) fn ensure_import_sources(&mut self) {
        if self.import_sources.is_some() {
            return;
        }
        self.import_sources = Some(Self::scan_import_sources());
    }

    /// Re-read every source, discarding the cache and any pending selection -
    /// a tick against a definition that is no longer there would import
    /// nothing and say nothing.
    pub(crate) fn refresh_import_sources(&mut self) {
        self.import_sources = Some(Self::scan_import_sources());
        self.import_subagent_selection.clear();
        self.import_workspace_selection.clear();
        self.import_result = None;
    }

    fn scan_import_sources() -> ImportSources {
        let subagents = SubagentTool::implemented().into_iter()
                                                   .filter_map(knot_core::import::provider)
                                                   .map(|provider| provider.definitions(None))
                                                   .fold(SubagentScan::default(), merge_scans);
        ImportSources { subagents,
                        skwad: knot_core::import::skwad::read() }
    }

    fn import_selected_definitions(&mut self, cx: &mut Context<Self>) {
        let Some(sources) = self.import_sources.as_ref()
        else {
            return;
        };
        let selected: Vec<_> =
            sources.subagents
                   .definitions
                   .iter()
                   .filter(|d| self.import_subagent_selection.contains(&d.source))
                   .cloned()
                   .collect();

        match knot_core::import::import_definitions(&mut self.settings, &selected) {
            Ok(mut result) => {
                // What the scan itself could not read belongs in the same
                // summary: the user asked to import from a source, and a file
                // that never made the list is part of that answer.
                result.unreadable.extend(self.import_sources
                                             .as_ref()
                                             .map(|s| s.subagents.unreadable.clone())
                                             .unwrap_or_default());
                self.import_result = Some(result);
            }
            Err(error) => eprintln!("failed to import subagent definitions: {error}"),
        }
        self.import_subagent_selection.clear();
        cx.notify();
    }

    fn import_selected_workspaces(&mut self, cx: &mut Context<Self>) {
        let Some(source) = self.import_sources.as_ref().map(|s| s.skwad.clone())
        else {
            return;
        };
        let selected: Vec<Uuid> = self.import_workspace_selection.iter().copied().collect();

        match knot_core::import::import_workspaces(&mut self.settings, &source, &selected) {
            Ok(mut result) => {
                result.unreadable.extend(source.unreadable.clone());
                self.import_result = Some(result);
            }
            Err(error) => eprintln!("failed to import Skwad workspaces: {error}"),
        }
        self.import_workspace_selection.clear();
        cx.notify();
    }

    pub(crate) fn render_import(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let sources = self.import_sources.as_ref();

        v_flex().gap_3()
                .child(self.render_import_refresh_row(cx))
                .child(self.render_import_personas_section(sources, cx))
                .child(self.render_import_skwad_section(sources, cx))
                .children(self.render_import_result(cx))
    }

    /// Re-scan every source. The tab scans once when entered, so a definition
    /// written while this window is open would otherwise not be offered until
    /// the window was reopened.
    fn render_import_refresh_row(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_window = cx.entity();
        h_flex().justify_end()
                .child(Self::icon_button("import-refresh",
                                         "icons/rotate-ccw.svg",
                                         knot_core::l10n::t("settings.import.refresh"),
                                         false).on_click(move |_, _, app| {
                                                   settings_window.update(app, |view, cx| {
                                                                      view.refresh_import_sources();
                                                                      cx.notify();
                                                                  });
                                               }))
    }

    /// Personas from a coding agent's subagent definitions.
    fn render_import_personas_section(&self, sources: Option<&ImportSources>,
                                      cx: &mut Context<Self>)
                                      -> impl IntoElement {
        let settings_window = cx.entity();
        let definitions = sources.map(|s| s.subagents.definitions.as_slice())
                                 .unwrap_or_default();
        let selected = self.import_subagent_selection.len();

        let body = if definitions.is_empty() {
            Self::import_empty_state("settings.import.personas_none", cx).into_any_element()
        }
        else {
            v_flex().gap_2()
                    .children(definitions.iter().enumerate().map(|(index, definition)| {
                                  let source = definition.source.clone();
                                  let checked = self.import_subagent_selection.contains(&source);
                                  let settings_window = settings_window.clone();
                                  Checkbox::new(("import-definition", index)).label(definition.name
                                                                                             .clone())
                          .checked(checked)
                          .on_click(move |checked, _, app| {
                              let source = source.clone();
                              let checked = *checked;
                              settings_window.update(app, |view, cx| {
                                                 if checked {
                                                     view.import_subagent_selection.insert(source);
                                                 }
                                                 else {
                                                     view.import_subagent_selection.remove(&source);
                                                 }
                                                 cx.notify();
                                             });
                          })
                              }))
                    .into_any_element()
        };

        Self::group(knot_core::l10n::t("settings.import.personas_title"))
            .child(Self::import_list("import-personas-list", body))
            .child(Self::import_action_row(
                "import-personas",
                selected,
                definitions.is_empty(),
                cx,
                move |view, cx| view.import_selected_definitions(cx),
            ))
    }

    /// Workspaces from a Skwad installation.
    fn render_import_skwad_section(&self, sources: Option<&ImportSources>,
                                   cx: &mut Context<Self>)
                                   -> impl IntoElement {
        let settings_window = cx.entity();
        let workspaces = sources.map(|s| s.skwad.workspaces.as_slice())
                                .unwrap_or_default();
        let selected = self.import_workspace_selection.len();

        let body = if workspaces.is_empty() {
            Self::import_empty_state("settings.import.workspaces_none", cx).into_any_element()
        }
        else {
            v_flex().gap_2()
                    .children(workspaces.iter().enumerate().map(|(index, workspace)| {
                                  let id = workspace.id;
                                  let checked = self.import_workspace_selection.contains(&id);
                                  let settings_window = settings_window.clone();
                                  Checkbox::new(("import-workspace", index))
                        .label(Self::workspace_import_label(workspace))
                        .checked(checked)
                        .on_click(move |checked, _, app| {
                            let checked = *checked;
                            settings_window.update(app, |view, cx| {
                                               if checked {
                                                   view.import_workspace_selection.insert(id);
                                               }
                                               else {
                                                   view.import_workspace_selection.remove(&id);
                                               }
                                               cx.notify();
                                           });
                        })
                              }))
                    .into_any_element()
        };

        Self::group(knot_core::l10n::t("settings.import.workspaces_title"))
            .child(Self::import_list("import-skwad-list", body))
            .child(Self::import_action_row(
                "import-skwad",
                selected,
                workspaces.is_empty(),
                cx,
                move |view, cx| view.import_selected_workspaces(cx),
            ))
    }

    /// A workspace's row label: its name and how many agents come with it, so
    /// the choice is made on what it actually brings across.
    pub(crate) fn workspace_import_label(workspace: &knot_core::Workspace) -> String {
        let agents = knot_core::l10n::pluralize(workspace.agent_ids.len() as u64,
                                                "count.agent",
                                                "count.agents");
        format!("{} ({agents})", workspace.name)
    }

    /// What a section shows in place of its list when its source holds
    /// nothing.
    fn import_empty_state(key: &str, cx: &Context<Self>) -> impl IntoElement {
        div().text_sm()
             .text_color(cx.theme().muted_foreground)
             .child(knot_core::l10n::t(key))
    }

    /// A section's scrollable list region, bounded so a long list scrolls in
    /// place rather than pushing the section below it off the window.
    fn import_list(id: impl Into<gpui_kit::ElementId>, body: gpui_kit::AnyElement)
                   -> impl IntoElement {
        div().id(id).max_h(px(200.)).overflow_y_scroll().child(body)
    }

    /// The Import button for one section, disabled until something is ticked
    /// so the control says what it needs rather than doing nothing when
    /// pressed.
    fn import_action_row(id: &'static str, selected: usize, source_empty: bool,
                         cx: &mut Context<Self>,
                         run: impl Fn(&mut Self, &mut Context<Self>) + 'static)
                         -> impl IntoElement {
        let settings_window = cx.entity();
        let label = if selected == 0 {
            knot_core::l10n::t("settings.import.import_selected")
        }
        else {
            knot_core::l10n::t_with("settings.import.import_count",
                                    &[("count", &selected.to_string())])
        };

        h_flex().justify_end().child(Button::new(id).label(label)
                                                    .primary()
                                                    .small()
                                                    .disabled(selected == 0 || source_empty)
                                                    .on_click(move |_, _, app| {
                                                        settings_window.update(app, |view, cx| {
                                                                           run(view, cx)
                                                                       });
                                                    }))
    }

    /// The summary of the last import, shown only after one has run.
    fn render_import_result(&self, cx: &Context<Self>) -> Option<impl IntoElement> {
        let result: &ImportResult = self.import_result.as_ref()?;
        let lines = result.summary_lines();
        let body = if lines.is_empty() {
            vec![knot_core::l10n::t("settings.import.nothing_to_do")]
        }
        else {
            lines
        };

        Some(Self::group(knot_core::l10n::t("settings.import.result_title"))
            .child(v_flex().gap_1().children(body.into_iter().map(|line| {
                div().text_sm().whitespace_normal().text_color(cx.theme().muted_foreground).child(line)
            }))))
    }
}

/// Fold one tool's scan into the running total, keeping definitions and
/// unreadable records in the order the tools were asked.
fn merge_scans(mut total: SubagentScan, next: SubagentScan) -> SubagentScan {
    total.definitions.extend(next.definitions);
    total.unreadable.extend(next.unreadable);
    total
}
