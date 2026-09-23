//! Which agent's composer the next frame will show
//! (`openspec/specs/acp-panel-ui`, "Selecting a Panel-mode agent focuses its
//! prompt input").
//!
//! This is the question `prepare_frame` has to answer before it can decide
//! whether the selection changed to an agent whose prompt input should take
//! focus. It is deliberately a pure function over facts already read out of
//! the store, not a method on the window: the branch order below has to stay
//! in step with `render/content.rs`, and a function taking the facts
//! explicitly is the only version of it that can be tested without a window.
//!
//! The order is `render/content.rs`'s own, and the reasoning for each step
//! belongs there rather than being restated here.

use uuid::Uuid;

/// What `render/content.rs` branches on when it picks the pane for the
/// selected agent.
///
/// Absence of this struct means the selected agent is not in the store,
/// which resolves the same way it does in the content pane: there is no
/// session to draw, so there is no composer either.
pub(crate) struct SelectedAgentFacts {
    pub(crate) id:            Uuid,
    /// The agent runs in Panel mode. Terminal-mode agents draw the terminal
    /// grid, which has no composer.
    pub(crate) is_panel_mode: bool,
    /// An open markdown file, which takes the content area ahead of either
    /// session pane.
    pub(crate) has_markdown:  bool,
    /// An open diagram, which takes the content area for the same reason.
    pub(crate) has_diagram:   bool,
    /// A deactivated agent draws the stopped placeholder instead of a pane.
    pub(crate) is_activated:  bool,
}

/// The agent whose prompt input the frame about to be drawn will show, or
/// `None` when it will show anything else.
///
/// `is_takeover` is the window showing the dashboard or the pull requests
/// view rather than an agent at all - the same flag `render` passes to
/// `prepare_frame`.
pub(crate) fn showing_composer(is_takeover: bool, selected: Option<&SelectedAgentFacts>)
                               -> Option<Uuid> {
    if is_takeover {
        return None;
    }
    let agent = selected?;
    if agent.has_markdown || agent.has_diagram {
        return None;
    }
    if !agent.is_activated {
        return None;
    }
    if !agent.is_panel_mode {
        return None;
    }
    Some(agent.id)
}
