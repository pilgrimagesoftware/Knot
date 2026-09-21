//! Creating an agent from this window, and the header facts a new or
//! selected one contributes.

use super::*;

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

    // SUPERSEDED: the inline new-agent form this submits was replaced by the
    // agent editor (`open_new_agent_dialog`). Kept with its sibling fields
    // until the form itself is removed - see the PR that added this note.
    #[allow(dead_code)]
    pub(super) fn create_agent(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let folder = self.new_agent_folder_input
                         .read(cx)
                         .value()
                         .trim()
                         .to_string();
        if folder.is_empty() || !PathBuf::from(&folder).is_dir() {
            self.error = Some("Choose an existing agent folder.".to_string());
            cx.notify();
            return false;
        }
        let name = self.new_agent_name_input
                       .read(cx)
                       .value()
                       .trim()
                       .to_string();
        let id = {
            let mut store = self.store.lock().unwrap();
            store.set_current_workspace(self.workspace_id);
            store.create(folder,
                         knot_agents::CreateOptions { name: (!name.is_empty()).then_some(name),
                                                      ..Default::default() })
        };
        if let Ok(store) = self.store.lock() {
            self.settings.saved_agents =
                store.saved_agents(self.settings.restore_conversation_on_launch);
            self.settings.saved_workspaces = store.saved_workspaces();
        }
        let _ = self.settings.persist();
        self.select_agent(id);
        self.show_new_agent = false;
        self.error = None;
        cx.update_entity(&self.new_agent_name_input, |input, input_cx| {
              input.clean(window, input_cx);
          });
        cx.update_entity(&self.new_agent_folder_input, |input, input_cx| {
              input.clean(window, input_cx);
          });
        cx.notify();
        true
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
                entity.update(app, |view, cx| {
                          view.select_agent(id);
                          view.view_mode = WorkspaceViewMode::Terminal;
                          view.ensure_session(id);
                          view.ensure_panel_session(id);
                          cx.notify();
                      });
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
