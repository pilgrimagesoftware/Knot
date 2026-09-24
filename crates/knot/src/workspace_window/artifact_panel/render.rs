//! The artifact panel itself: its toolbar, and the one or two sections below
//! it (`openspec/specs/artifact-panel`).
//!
//! Ports `ArtifactPanelView.body` and its two layout arms. The single-section
//! case fills the panel with no divider and no chevron; the dual case splits
//! the height by [`super::layout::section_heights`] and puts a draggable
//! divider between, unless either section is collapsed - then there is no
//! split left to set.

use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::resizable::ResizableState;
use gpui_kit::component::resizable::resizable_panel;
use gpui_kit::component::resizable::v_resizable;
use gpui_kit::div;
use gpui_kit::px;
use uuid::Uuid;

use super::layout;
use super::section::SectionChrome;
use super::state::ArtifactSnapshot;
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// What `id`'s artifact fields hold right now.
    ///
    /// Read once and passed around rather than re-locking the store per
    /// section: the panel's shape depends on both fields together, and two
    /// reads could disagree across a write from the MCP server's thread.
    pub(in crate::workspace_window) fn artifact_snapshot(&self, id: Uuid) -> ArtifactSnapshot {
        let store = self.store.lock();
        let Some(agent) = store.agent(id)
        else {
            return ArtifactSnapshot::default();
        };
        ArtifactSnapshot { markdown:  agent.markdown_file.clone(),
                           maximized: agent.markdown_maximized,
                           mermaid:   agent.mermaid_source.clone(), }
    }

    /// The artifact panel for `id`, or `None` when that agent has neither an
    /// open file nor an open diagram.
    pub(in crate::workspace_window) fn render_artifact_panel(&mut self, id: Uuid,
                                                             snapshot: &ArtifactSnapshot,
                                                             cx: &mut Context<Self>)
                                                             -> Option<gpui_kit::AnyElement> {
        let mermaid = snapshot.mermaid.clone().map(|source| {
                                                  let title = {
                                                      let store = self.store.lock();
                                                      store.agent(id).and_then(|agent| {
                                                                         agent.mermaid_title.clone()
                                                                     })
                                                  };
                                                  (source, title)
                                              });
        let markdown = snapshot.markdown.clone();
        if markdown.is_none() && mermaid.is_none() {
            return None;
        }
        let arrangement = self.artifact_arrangement(id);
        // Collapsible only when both are open: a lone section has nothing to
        // give its height to, so the chevron would offer a gesture that
        // cannot do anything.
        let both = markdown.is_some() && mermaid.is_some();
        let markdown_collapsed = both && arrangement.markdown_collapsed;
        let mermaid_collapsed = both && arrangement.mermaid_collapsed;

        let markdown_section = markdown.map(|file| {
                                           let chrome = SectionChrome { collapsible: both,
                                                                        collapsed:
                                                                            markdown_collapsed, };
                                           self.render_markdown_pane(id, &file, chrome, cx)
                                       });
        let mermaid_section =
            mermaid.map(|(source, title)| {
                       let chrome = SectionChrome { collapsible: both,
                                                    collapsed:   mermaid_collapsed, };
                       self.render_mermaid_pane(id, &source, title.as_deref(), chrome, cx)
                   });

        let sections = match (markdown_section, mermaid_section) {
            // Both open and neither collapsed: the one case with a split to
            // set, so the one case that gets a divider.
            (Some(top), Some(bottom)) if !markdown_collapsed && !mermaid_collapsed => {
                self.artifact_split(id, arrangement.split, top, bottom, cx)
            }
            // Both open, one collapsed: a header and the rest, with no
            // divider - there is no split left to set.
            (Some(top), Some(bottom)) => {
                let (top_height, bottom_height) = layout::section_heights(REFERENCE_HEIGHT,
                                                                          arrangement.split,
                                                                          markdown_collapsed,
                                                                          mermaid_collapsed);
                v_flex().size_full()
                        .min_h_0()
                        .child(collapsed_or_filling(top, top_height, markdown_collapsed))
                        .child(collapsed_or_filling(bottom, bottom_height, mermaid_collapsed))
                        .into_any_element()
            }
            // One open: it fills the panel.
            (Some(only), None) | (None, Some(only)) => v_flex().size_full()
                                                               .min_h_0()
                                                               .child(only)
                                                               .into_any_element(),
            (None, None) => return None,
        };
        Some(v_flex().size_full()
                     .bg(cx.theme().background)
                     .child(self.artifact_toolbar(id, arrangement.expanded, cx))
                     .child(div().flex_1().min_h_0().child(sections))
                     .into_any_element())
    }

    /// The two sections with a draggable divider between them.
    ///
    /// Seeded against [`REFERENCE_HEIGHT`] rather than the panel's real one,
    /// which is not known while the tree is being built: the group scales the
    /// two to whatever height it is given, so only their proportion matters
    /// here. The ratio written back on a drag comes from the sizes the group
    /// reports, which are real pixels.
    fn artifact_split(&mut self, id: Uuid, split: f32, top: gpui_kit::AnyElement,
                      bottom: gpui_kit::AnyElement, cx: &mut Context<Self>)
                      -> gpui_kit::AnyElement {
        let (top_height, bottom_height) =
            layout::section_heights(REFERENCE_HEIGHT, split, false, false);
        let state = self.artifact_split_resize
                        .entry(id)
                        .or_insert_with(|| cx.new(|_| ResizableState::default()))
                        .clone();
        v_resizable("artifact-sections").with_state(&state)
                                        .on_resize(cx.listener(move |view,
                                                          state: &Entity<ResizableState>,
                                                          _window,
                                                          cx| {
                                            let sizes = state.read(cx).sizes();
                                            let (Some(first), Some(second)) =
                                                (sizes.first(), sizes.get(1))
                                            else {
                                                return;
                                            };
                                            let total =
                                                f32::from(*first) + f32::from(*second);
                                            if total > 0. {
                                                view.set_artifact_split(id,
                                                                        f32::from(*first)
                                                                        / total);
                                            }
                                        }))
                                        .child(resizable_panel().size(px(top_height))
                                                                .child(top))
                                        .child(resizable_panel().size(px(bottom_height))
                                                                .child(bottom))
                                        .into_any_element()
    }

    /// The panel's toolbar: its title, the expand toggle, and close-all.
    fn artifact_toolbar(&self, id: Uuid, expanded: bool, cx: &mut Context<Self>)
                        -> gpui_kit::AnyElement {
        let toggle_label = if expanded {
            "artifact_panel.collapse"
        }
        else {
            "artifact_panel.expand"
        };
        h_flex().w_full()
                .flex_shrink_0()
                .items_center()
                .justify_between()
                .gap_2()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(cx.theme().border)
                .child(div().flex_1()
                            .min_w_0()
                            .font_semibold()
                            .child(knot_core::l10n::t("artifact_panel.title")))
                .child(Button::new(("artifact-panel-expand", id.as_u128() as u64))
                    .icon(if expanded {
                        IconName::Minimize
                    }
                    else {
                        IconName::Maximize
                    })
                    .ghost()
                    .small()
                    .tooltip(knot_core::l10n::t(toggle_label))
                    .on_click(cx.listener(move |view, _, _window, cx| {
                        view.toggle_artifact_expanded(id);
                        cx.notify();
                    })))
                .child(Button::new(("artifact-panel-close-all", id.as_u128() as u64))
                    .icon(IconName::Close)
                    .ghost()
                    .small()
                    .tooltip(knot_core::l10n::t("artifact_panel.close_all"))
                    .on_click(cx.listener(move |view, _, _window, cx| {
                        {
                            let mut store = view.store.lock();
                            if let Err(error) = store.clear_markdown_panel(id) {
                                eprintln!("failed to close the markdown section: {error}");
                            }
                            if let Err(error) = store.clear_mermaid_panel(id) {
                                eprintln!("failed to close the diagram section: {error}");
                            }
                        }
                        cx.notify();
                    })))
                .into_any_element()
    }
}

/// The height the split is seeded against.
///
/// Any consistent value works: the resizable group scales its panels to the
/// height it is actually given, so this only has to carry the proportion.
const REFERENCE_HEIGHT: f32 = 1000.;

/// A section drawn at its collapsed header height, or filling what is left.
fn collapsed_or_filling(section: gpui_kit::AnyElement, height: f32, collapsed: bool)
                        -> gpui_kit::AnyElement {
    if collapsed {
        div().flex_shrink_0()
             .h(px(height))
             .child(section)
             .into_any_element()
    }
    else {
        div().flex_1().min_h_0().child(section).into_any_element()
    }
}
