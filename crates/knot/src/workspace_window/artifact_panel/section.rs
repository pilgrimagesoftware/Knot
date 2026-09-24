//! The header both artifact sections wear, and the chrome that says which
//! one it is (`openspec/specs/artifact-panel`, "Either section can be
//! collapsed to its header").
//!
//! Shared rather than written twice: the two sections differ in what they
//! draw below the header, and in nothing above it. Ports the header arms of
//! `MarkdownPanelView` and `MermaidPanelView`, which take the same
//! `isCollapsible` / `isCollapsed` pair.

use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt;
use gpui_kit::base::h_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::div;
use uuid::Uuid;

use crate::app_support::single_line;
use crate::workspace_window::WorkspaceWindow;

/// Which of the two sections a header belongs to.
///
/// An enum rather than a bool: the two differ in which collapse flag they
/// toggle and in the element ids they need, and a bool at three call sites
/// reads as nothing in particular.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::workspace_window) enum Section {
    Markdown,
    Mermaid,
}

impl Section {
    const fn id_prefix(self) -> &'static str {
        match self {
            Self::Markdown => "markdown",
            Self::Mermaid => "mermaid",
        }
    }

    /// The close control's element id.
    ///
    /// A static string per variant rather than one formatted from
    /// [`Self::id_prefix`]: `ElementId` is built from a `&'static str` and a
    /// number, and a `String` does not convert.
    const fn close_id(self) -> &'static str {
        match self {
            Self::Markdown => "markdown-pane-close",
            Self::Mermaid => "mermaid-pane-close",
        }
    }
}

/// What a section's header needs to know about its own state.
///
/// `collapsible` is false whenever the section is the only one open: there is
/// nothing to trade height with, so a chevron would offer a gesture that
/// cannot do anything.
#[derive(Debug, Clone, Copy, Default)]
pub(in crate::workspace_window) struct SectionChrome {
    pub(in crate::workspace_window) collapsible: bool,
    pub(in crate::workspace_window) collapsed:   bool,
}

impl WorkspaceWindow {
    /// A section's header: its chevron when collapsible, its title, and its
    /// own close control.
    ///
    /// The close control is the section's, not the panel's - it closes this
    /// artifact and leaves the other's alone, which is what distinguishes it
    /// from the toolbar's close-all.
    pub(in crate::workspace_window) fn artifact_section_header(&self, id: Uuid,
                                                               section: Section, title: &str,
                                                               chrome: SectionChrome,
                                                               cx: &mut Context<Self>)
                                                               -> gpui_kit::AnyElement {
        let prefix = section.id_prefix();
        h_flex().w_full()
                .flex_shrink_0()
                .items_center()
                .justify_between()
                .gap_2()
                .px_3()
                .py_2()
                .border_b_1()
                .border_color(cx.theme().border)
                .children(chrome.collapsible.then(|| {
                              let label = if chrome.collapsed {
                                  "artifact_panel.expand_section"
                              }
                              else {
                                  "artifact_panel.collapse_section"
                              };
                              Button::new(("artifact-section-chevron", section as usize))
                        .icon(if chrome.collapsed {
                            IconName::ChevronRight
                        }
                        else {
                            IconName::ChevronDown
                        })
                        .ghost()
                        .small()
                        .tooltip(knot_core::l10n::t(label))
                        .on_click(cx.listener(move |view, _, _window, cx| {
                            match section {
                                Section::Markdown => {
                                    view.toggle_artifact_markdown_collapsed(id);
                                }
                                Section::Mermaid => {
                                    view.toggle_artifact_mermaid_collapsed(id);
                                }
                            }
                            cx.notify();
                        }))
                          }))
                .child(div().flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .font_semibold()
                            .child(single_line(title)))
                .child(Button::new((section.close_id(), id.as_u128() as u64))
                    .icon(IconName::Close)
                    .ghost()
                    .small()
                    .tooltip(knot_core::l10n::t("panel.close"))
                    .on_click(cx.listener(move |view, _, _window, cx| {
                        {
                            let mut store = view.store.lock();
                            let closed = match section {
                                Section::Markdown => store.clear_markdown_panel(id),
                                Section::Mermaid => store.clear_mermaid_panel(id),
                            };
                            if let Err(error) = closed {
                                eprintln!("failed to close the {prefix} section: {error}");
                            }
                        }
                        cx.notify();
                    })))
                .into_any_element()
    }
}
