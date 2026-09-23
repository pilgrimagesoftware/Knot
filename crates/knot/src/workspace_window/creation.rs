//! Creating an agent from this window, and the header facts a new or
//! selected one contributes.

use std::sync::Arc;

use gpui_kit::App;
use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::component::ActiveTheme;
use uuid::Uuid;

use crate::agent_editor::AgentEditorRequest;
use crate::agent_editor::AgentPrefill;
use crate::agent_editor::open_agent_editor;
use crate::app_state;
use crate::workspace_window::WorkspaceWindow;

/// Parameters for [`open_agent_editor`], grouped to keep the function's
/// argument count in check.
pub(crate) struct SelectedAgentHeader {
    pub(crate) avatar:       String,
    pub(crate) name:         String,
    pub(crate) folder:       String,
    pub(crate) header_title: String,
    /// Snapshotted with the rest of the header but not drawn: the header
    /// shows the agent's identity, and its type is already implied by the
    /// avatar. Kept because the struct is the header's whole snapshot.
    #[allow(dead_code)]
    pub(crate) agent_type:   String,
    /// The agent's state and its diff stat *lookup*: the outer `Option`
    /// is whether the first refresh has finished, the inner one whether
    /// it found a repository. Only a missing lookup means "still
    /// working" - a folder that is not a git checkout must not sit on
    /// "Getting stats…" for the life of the window.
    pub(crate) state:        Option<(knot_agents::AgentState, Option<Option<knot_git::DiffStats>>)>,
}

impl WorkspaceWindow {
    /// Finds the declared config option matching one of `categories`
    /// (case-insensitive), for bucketing the agent's arbitrary option list
    /// into the input area's three fixed selector slots.
    pub(crate) fn find_config_option<'a>(options: &'a [knot_acp::ConfigOption],
                                         categories: &[&str])
                                         -> Option<&'a knot_acp::ConfigOption> {
        options.iter().find(|option| {
                          option.kind == "select"
                          && option.category.as_deref().is_some_and(|category| {
                                                           categories
                        .iter()
                        .any(|candidate| candidate.eq_ignore_ascii_case(category))
                                                       })
                      })
    }

    pub(super) fn open_new_agent_dialog(&mut self, cx: &mut Context<Self>) {
        open_agent_editor(Arc::clone(&self.store),
                          self.settings.clone(),
                          AgentEditorRequest { workspace_id: self.workspace_id,
                                               prefill:      AgentPrefill::default(),
                                               insert_after: None,
                                               edit_target:  None, },
                          Self::select_and_focus_created_agent(cx),
                          cx);
    }

    /// An `on_created` callback for [`open_agent_editor`] that selects the
    /// new agent (and switches out of the dashboard, if it was open) so it
    /// becomes the visible agent in the sidebar and content pane, matching
    /// how tapping an existing agent already behaves.
    pub(super) fn select_and_focus_created_agent(
        cx: &mut Context<Self>)
        -> impl Fn(Uuid, &mut Window, &mut App) + 'static {
        let weak = cx.entity().downgrade();
        move |id, _window, app| {
            if let Some(entity) = weak.upgrade() {
                entity.update(app, |view, cx| view.reveal_agent(id, cx));
            }
        }
    }

    /// The selected agent's diff stat, with only the figures colored -
    /// additions green, deletions red, the changed-file count blue - and
    /// the words around them left muted. The count's noun goes through
    /// `l10n::plural_noun` rather than a local `if count == 1`, so the
    /// word (and its form) comes from the locale catalog.
    pub(super) fn render_diff_stats(stats: &knot_git::DiffStats, font_family: String,
                                    font_size: gpui_kit::Pixels, cx: &Context<Self>)
                                    -> gpui_kit::AnyElement {
        app_state::diff_stats_row(stats, cx.theme().muted_foreground).font_family(font_family)
                                                                     .text_size(font_size)
                                                                     .into_any_element()
    }
}
