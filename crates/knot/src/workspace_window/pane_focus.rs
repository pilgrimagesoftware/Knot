//! Which of the selected agent's input targets the next frame will show
//! (`openspec/specs/acp-panel-ui`, "Selecting a Panel-mode agent focuses its
//! prompt input", and `openspec/specs/terminal-input`, "Selecting a
//! Terminal-mode agent focuses its terminal surface").
//!
//! This is the question `prepare_frame` has to answer before it can decide
//! whether the selection changed to an agent whose input should take focus.
//! It is deliberately a pure function over facts already read out of the
//! store, not a method on the window: the branch order below has to stay in
//! step with `render/content.rs`, and a function taking the facts explicitly
//! is the only version of it that can be tested without a window.
//!
//! One function over both targets rather than two predicates with a latch
//! each. Selecting a Panel-mode agent and then a Terminal-mode one has to
//! read as a transition for the terminal, and two independent latches each
//! see only their own half of that.
//!
//! The order is `render/content.rs`'s own, and the reasoning for each step
//! belongs there rather than being restated here.

use uuid::Uuid;

/// What `render/content.rs` branches on when it picks the pane for the
/// selected agent.
///
/// Absence of this struct means the selected agent is not in the store,
/// which resolves the same way it does in the content pane: there is no
/// session to draw, so there is nothing to focus either.
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
    /// The agent's session has produced a grid, so the content pane draws
    /// the terminal surface rather than the "Starting terminal…"
    /// placeholder.
    ///
    /// Only meaningful for a Terminal-mode agent, and the reason the
    /// terminal's answer is not simply "is it selected": the placeholder
    /// tracks no focus handle, so focus taken over it lands outside the
    /// element tree and `prepare_frame`'s own fallback moves it to the
    /// window root on the same frame - with the latch already stored, and
    /// nothing left to retry.
    pub(crate) has_live_grid: bool,
}

/// The input target the frame about to be drawn will show.
///
/// Carries the agent id in both variants because the latch compares whole
/// answers: the same target for a different agent is a transition, and a
/// different target for the same agent is one too.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FocusTarget {
    /// The agent's prompt input, in a Panel-mode conversation.
    Composer(Uuid),
    /// The agent's terminal surface.
    Terminal(Uuid),
}

impl FocusTarget {
    /// The agent this target belongs to.
    pub(crate) fn agent(self) -> Uuid {
        match self {
            Self::Composer(id) | Self::Terminal(id) => id,
        }
    }
}

/// The input target the frame about to be drawn will show, or `None` when it
/// will show something with nothing to type into.
///
/// `is_takeover` is the window showing the dashboard or the pull requests
/// view rather than an agent at all - the same flag `render` passes to
/// `prepare_frame`.
pub(crate) fn focus_target(is_takeover: bool, selected: Option<&SelectedAgentFacts>)
                           -> Option<FocusTarget> {
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
    if agent.is_panel_mode {
        return Some(FocusTarget::Composer(agent.id));
    }
    agent.has_live_grid
         .then_some(FocusTarget::Terminal(agent.id))
}
