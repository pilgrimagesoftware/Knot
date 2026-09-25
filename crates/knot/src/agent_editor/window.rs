//! The agent editor dialog: what opening one takes, the window it opens, and
//! the form state it holds while the user fills it in.
//!
//! What submitting it does is `super::submit`; how it is drawn is
//! `super::render`.

use std::sync::Arc;

use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::Entity;
use gpui_kit::Subscription;
use gpui_kit::Window;
use gpui_kit::component::Root;
use gpui_kit::component::input::InputEvent;
use gpui_kit::component::input::InputState;
use gpui_kit::component::input::TextareaState;
use parking_lot::Mutex;
use uuid::Uuid;

use super::fields::AgentPrefill;
use crate::app_support::observe_system_appearance;
use crate::consts;
use crate::window_options::agent_window_options;

pub(crate) struct AgentEditorRequest {
    pub(crate) workspace_id: Uuid,
    /// What the new agent starts from. Ignored when `edit_target` is set.
    pub(crate) prefill:      AgentPrefill,
    /// Where to insert a newly created agent. Ignored when `edit_target` is
    /// set - editing never moves an agent's position.
    pub(crate) insert_after: Option<Uuid>,
    /// `Some(id)` opens the dialog in edit mode for that existing agent
    /// (prefilled from it, submitting via `AgentStore::edit`) instead of
    /// creating a new one.
    pub(crate) edit_target:  Option<Uuid>,
}

/// Opens the agent editor dialog, in create or edit mode depending on
/// `request.edit_target`. `on_created` is called with the new (create mode)
/// or edited (edit mode) agent's id once the dialog is submitted.
pub(crate) fn open_agent_editor(store: Arc<Mutex<knot_agents::AgentStore>>,
                                request: AgentEditorRequest,
                                on_created: impl Fn(Uuid, &mut Window, &mut App) + 'static,
                                cx: &mut App) {
    let AgentEditorRequest { workspace_id,
                             prefill,
                             insert_after,
                             edit_target, } = request;
    // No re-read here any more. This loaded from disk because every window
    // held its own snapshot taken when it opened, so a persona added or
    // renamed since - in a different window, which persists to disk - was
    // missing from the picker, which is the whole content of this dialog's
    // persona field. The surface is never stale, so there is nothing to
    // re-read.
    let editing = edit_target.and_then(|id| store.lock().agent(id).cloned());
    let title = if editing.is_some() {
        knot_core::l10n::t("agent_editor.title_edit")
    }
    else {
        knot_core::l10n::t("agent_editor.title_new")
    };
    let options = agent_window_options(&title, cx);
    let _ =
        cx.open_window(options, move |window, cx| {
              // Every window tracks the OS appearance, so a light/dark flip
              // re-resolves the system palette and repaints.
              observe_system_appearance(window);
              let name_input = cx.new(|cx| {
                                     InputState::new(window, cx).placeholder(knot_core::l10n::t("agent_editor.name"))
                                                   .default_value(editing.as_ref()
                                                                         .map(|a| a.name.clone())
                                                                         .or_else(|| {
                                                                             prefill.name.clone()
                                                                         })
                                                                         .unwrap_or_default())
                                 });
              let shell_command_input =
                  cx.new(|cx| InputState::new(window, cx).placeholder(knot_core::l10n::t("agent_editor.shell_command")));
              let description_input = cx.new(|cx| {
                  InputState::new(window, cx)
                      .placeholder(knot_core::l10n::t("agent_editor.description_placeholder"))
                      .default_value(editing.as_ref()
                                            .map(|a| a.description.clone())
                                            .unwrap_or_default())
              });
              // Comma-separated, because a tag set is short and typing one
              // is faster than managing a chip list for it. `Capabilities`
              // normalizes whatever is typed, so spacing and case here do
              // not matter.
              let capabilities_input = cx.new(|cx| {
                  InputState::new(window, cx)
                      .placeholder(knot_core::l10n::t("agent_editor.capabilities_placeholder"))
                      .default_value(editing.as_ref()
                                            .map(|a| {
                                                a.capabilities
                                                 .iter()
                                                 .cloned()
                                                 .collect::<Vec<String>>()
                                                 .join(", ")
                                            })
                                            .unwrap_or_default())
              });
              let avatar_input = cx.new(|cx| {
                                       InputState::new(window, cx).default_value(
                editing
                    .as_ref()
                    .map(|a| a.avatar.clone())
                    .or_else(|| prefill.avatar.clone())
                    .unwrap_or_else(|| consts::DEFAULT_AGENT_AVATAR.to_string()),
            )
                                   });
              let stored_startup = editing.as_ref()
                                          .map(|a| a.startup_prompt.clone())
                                          .unwrap_or_else(|| prefill.startup_prompt.clone());
              let (startup_choice, startup_text) =
                  crate::startup_choice::initial_choice(stored_startup.as_ref());
              let startup_custom_input = cx.new(|cx| {
                  TextareaState::new(window, cx)
                      .placeholder(knot_core::l10n::t("agent_editor.startup_prompt_placeholder"))
                      .default_value(startup_text)
              });
              let view = cx.new(|cx| {
                               let avatar_subscription = cx.subscribe_in(
                &avatar_input,
                window,
                |this: &mut AgentEditor, avatar_input, event, window, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.clamp_avatar_to_one_character(avatar_input, window, cx);
                    }
                },
            );
                               let name_subscription =
                                   cx.subscribe(&name_input,
                                                |_: &mut AgentEditor, _, event, cx| {
                                                    if matches!(event, InputEvent::Change) {
                                                        cx.notify();
                                                    }
                                                });
                               let startup_subscription =
                                   cx.subscribe(&startup_custom_input,
                                                |_: &mut AgentEditor, _, event, cx| {
                                                    if matches!(event, InputEvent::Change) {
                                                        cx.notify();
                                                    }
                                                });
                               let persona_id = editing.as_ref()
                                                       .map(|a| a.persona_id)
                                                       .unwrap_or(prefill.persona_id);
                               AgentEditor { store,
                                          workspace_id,
                                          name_input,
                                          shell_command_input,
                                          avatar_input,
                                          description_input,
                                          capabilities_input,
                                          cost_tier:
                                              editing.as_ref()
                                                     .map(|a| a.cost_tier)
                                                     .unwrap_or_default(),
                                          _avatar_subscription: avatar_subscription,
                                          _name_subscription: name_subscription,
                                          folder_path: editing.as_ref()
                                                              .map(|a| a.folder.clone())
                                                              .or_else(|| prefill.folder.clone())
                                                              .unwrap_or_default(),
                                          agent_type:
                                              editing.as_ref()
                                                     .map(|a| a.agent_type.clone())
                                                     .or_else(|| prefill.agent_type.clone())
                                                     .unwrap_or_else(|| knot_core::agent_type::DEFAULT.to_string()),
                                          persona_id,
                                          activation_mode:
                                              editing.as_ref()
                                                     .map(|a| a.activation_mode)
                                                     .unwrap_or(knot_core::ActivationMode::Passive),
                                          original_persona_id: persona_id,
                                          startup_choice,
                                          startup_custom_input,
                                          _startup_subscription: startup_subscription,
                                          prefill,
                                          insert_after,
                                          edit_target,
                                          on_created: Box::new(on_created),
                                          error: None }
                           });
              cx.new(|cx| Root::new(view, window, cx))
          });
}

pub(crate) struct AgentEditor {
    pub(super) store:                 Arc<Mutex<knot_agents::AgentStore>>,
    pub(super) workspace_id:          Uuid,
    pub(super) name_input:            Entity<InputState>,
    pub(super) shell_command_input:   Entity<InputState>,
    pub(super) avatar_input:          Entity<InputState>,
    pub(super) description_input:     Entity<InputState>,
    pub(super) capabilities_input:    Entity<InputState>,
    pub(super) cost_tier:             knot_core::CostTier,
    pub(super) _avatar_subscription:  Subscription,
    pub(super) _name_subscription:    Subscription,
    pub(super) folder_path:           String,
    pub(super) agent_type:            String,
    pub(super) persona_id:            Option<Uuid>,
    /// When the agent starts on its own. `Passive` when creating - the
    /// deliberate disagreement with the `Active` a record carrying no
    /// stored mode loads as (see `knot_core::SavedAgent::activation_mode`)
    /// - and the agent's own mode when editing.
    pub(super) activation_mode:       knot_core::ActivationMode,
    /// Snapshot of `persona_id` when the dialog opened, so `save_edit` can
    /// tell `AgentStore::edit` whether the persona actually changed
    /// (`EditRequest::persona_changed`) rather than always forcing a
    /// restart.
    pub(super) original_persona_id:   Option<Uuid>,
    /// Which form of startup prompt is selected. See
    /// `openspec/specs/agent-editor-ui/spec.md`, "Startup prompt control".
    pub(super) startup_choice:        crate::startup_choice::StartupChoice,
    /// The custom startup prompt's text, kept while another choice is
    /// selected so switching back does not lose it.
    pub(super) startup_custom_input:  Entity<TextareaState>,
    /// Re-renders on each edit so the unknown-variable warning keeps up.
    pub(super) _startup_subscription: Subscription,
    /// What this editor was opened to derive the new agent from - carries
    /// the owner for a companion and the session for a fork, neither of
    /// which the form itself can express.
    pub(super) prefill:               AgentPrefill,
    pub(super) insert_after:          Option<Uuid>,
    /// `Some(id)` when editing an existing agent instead of creating one -
    /// gates prefill, the submit button's label/handler, and whether the
    /// (edit-unsupported) shell-command field shows at all.
    pub(super) edit_target:           Option<Uuid>,
    pub(super) on_created:            Box<AgentCreatedCallback>,
    pub(super) error:                 Option<String>,
}

pub(crate) type AgentCreatedCallback = dyn Fn(Uuid, &mut Window, &mut App);
