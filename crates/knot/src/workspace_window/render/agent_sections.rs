//! The row of disclosure sections below the agent's session pane.
//!
//! MCP servers and processes share one row while both are shut. Both answer
//! "what is this agent touching that I cannot see", both collapse to a single
//! header, and stacking two collapsed headers spends two rows of vertical
//! space on what reads as one band of status - space taken directly from the
//! session pane above them.
//!
//! Opening either one changes that. An open section needs the full width for
//! its rows - a server's address or a process's command line beside a
//! neighbour has nowhere to go - so the row becomes a stack, and the section
//! that was not opened shuts. Its header stays, full width, because a header
//! you cannot see is a section you cannot reopen.
//!
//! The order is fixed in both layouts, MCP first. A section that moved when
//! its neighbour opened would make the user look for it.

use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::ActiveTheme;
use gpui_kit::{Context, IntoElement, ParentElement, Styled};

use crate::workspace_window::WorkspaceWindow;

/// How the two sections are arranged this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SectionLayout {
    /// Both shut: side by side, each taking half the width.
    Row,
    /// One open: stacked, each taking the full width.
    Stacked,
}

impl SectionLayout {
    pub(super) fn is_stacked(self) -> bool {
        self == Self::Stacked
    }
}

impl WorkspaceWindow {
    /// The sections, or `None` when neither belongs on screen.
    pub(super) fn agent_sections_row(&mut self, cx: &mut Context<Self>)
                                     -> Option<gpui_kit::AnyElement> {
        let layout = self.agent_sections_layout();

        let mcp = self.mcp_servers_section(layout, cx);
        let processes = self.processes_section(layout, cx);

        if mcp.is_none() && processes.is_none() {
            return None;
        }

        // The top border belongs out here: drawn on each section instead, two
        // adjacent sections draw the same line twice and any difference in
        // their padding shows up as a step in it.
        let container = if layout.is_stacked() {
            v_flex()
        }
        else {
            h_flex().items_start()
        };

        Some(container.w_full()
                      .flex_shrink_0()
                      .border_t_1()
                      .border_color(cx.theme().border)
                      .children(mcp)
                      .children(processes)
                      .into_any_element())
    }

    /// Whether either section is open, and so whether they stack.
    fn agent_sections_layout(&self) -> SectionLayout {
        let Some(agent_id) = self.selected_agent
        else {
            return SectionLayout::Row;
        };

        let mcp_open = self.mcp_section(agent_id)
                           .is_some_and(|section| section.expanded);
        let processes_open = self.process_section(agent_id)
                                 .is_some_and(crate::agent_processes::ProcessSection::is_expanded);

        if mcp_open || processes_open {
            return SectionLayout::Stacked;
        }

        SectionLayout::Row
    }
}
