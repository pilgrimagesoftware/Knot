use std::path::PathBuf;
mod render;
use std::sync::Arc;

use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::PathPromptOptions;
use gpui_kit::Subscription;
use gpui_kit::Window;
use gpui_kit::component::Root;
use gpui_kit::component::input::InputEvent;
use gpui_kit::component::input::InputState;
use parking_lot::Mutex;
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;

// macOS-only: the module it names is `cfg(target_os = "macos")`, and so
// is every use of it here.
#[cfg(target_os = "macos")]
use crate::app_support::native_character_picker;
use crate::app_support::observe_system_appearance;
use crate::consts;
use crate::window_options::agent_window_options;

/// What a new agent starts out as when the editor is opened to create one
/// *derived* from an existing agent - a fork, or a companion of it -
/// rather than from nothing. Mirrors the Swift reference's `AgentPrefill`
/// (`Skwad/Models/Agent.swift`). Every field is ignored when the editor
/// opens in edit mode.
#[derive(Debug, Clone, Default)]
pub(crate) struct AgentPrefill {
    pub(crate) name:         Option<String>,
    pub(crate) avatar:       Option<String>,
    /// Folder for the new agent, the way the Swift reference's
    /// `addAgent(to:)` does from a dashboard's "Add Agent" tile: copied
    /// from an existing agent in the workspace.
    pub(crate) folder:       Option<String>,
    pub(crate) agent_type:   Option<String>,
    pub(crate) persona_id:   Option<Uuid>,
    /// The owner, when creating a companion of an existing agent.
    pub(crate) created_by:   Option<Uuid>,
    pub(crate) is_companion: bool,
    /// The session the new agent continues, when forking. The created
    /// agent carries it as both its session id and its resume target, with
    /// the fork flag set, so the fork picks the conversation up rather than
    /// taking it over.
    pub(crate) session_id:   Option<String>,
}

/// The agent type a submitted create actually uses.
///
/// A companion is a shell agent by definition - `create_shell_companion`
/// hardcodes the type, and the MCP `create-agent` tool refuses `companion`
/// for anything else - so a companion-creating editor ignores whatever the
/// type field holds rather than creating a companion the rest of the stack
/// rejects.
pub(crate) fn created_agent_type(creating_a_companion: bool, chosen: &str) -> String {
    if creating_a_companion {
        knot_core::agent_type::SHELL.to_string()
    }
    else {
        chosen.to_string()
    }
}

/// The personas the editor's picker offers.
///
/// `Settings::personas` is the *stored* list, which keeps soft-deleted
/// system personas so persistence can tell "deleted" from "never
/// installed". Selection reads the active list instead, per
/// `openspec/specs/personas/spec.md` - "Active versus stored personas" -
/// which drops deleted entries and sorts by name.
pub(crate) fn persona_choices(settings: &knot_core::Settings) -> Vec<knot_core::Persona> {
    settings.active_personas().into_iter().cloned().collect()
}

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
                                settings: knot_core::Settings, request: AgentEditorRequest,
                                on_created: impl Fn(Uuid, &mut Window, &mut App) + 'static,
                                cx: &mut App) {
    let AgentEditorRequest { workspace_id,
                             prefill,
                             insert_after,
                             edit_target, } = request;
    // Re-read from disk rather than trusting the caller's copy. Every
    // window holds its own `Settings` snapshot taken when it opened, and
    // personas are edited in a different window that persists to disk -
    // so a persona added or renamed since this window opened was missing
    // from the picker, which is the whole content of this dialog's
    // persona field. Falls back to the caller's snapshot if the file
    // can't be read.
    let settings = knot_core::Settings::load().unwrap_or(settings);
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
                               let persona_id = editing.as_ref()
                                                       .map(|a| a.persona_id)
                                                       .unwrap_or(prefill.persona_id);
                               AgentEditor { store,
                                          settings,
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
                                          prefill,
                                          insert_after,
                                          edit_target,
                                          on_created: Box::new(on_created),
                                          error: None }
                           });
              cx.new(|cx| Root::new(view, window, cx))
          });
}

/// Splits the capabilities field into tags. `Capabilities` trims,
/// lowercases and drops empties, so this only has to decide where one tag
/// ends and the next begins.
pub(crate) fn parse_capability_tags(raw: &str) -> knot_core::Capabilities {
    raw.split(',').collect()
}

pub(crate) struct AgentEditor {
    store:                Arc<Mutex<knot_agents::AgentStore>>,
    settings:             knot_core::Settings,
    workspace_id:         Uuid,
    name_input:           Entity<InputState>,
    shell_command_input:  Entity<InputState>,
    avatar_input:         Entity<InputState>,
    description_input:    Entity<InputState>,
    capabilities_input:   Entity<InputState>,
    cost_tier:            knot_core::CostTier,
    _avatar_subscription: Subscription,
    _name_subscription:   Subscription,
    folder_path:          String,
    agent_type:           String,
    persona_id:           Option<Uuid>,
    /// When the agent starts on its own. `Passive` when creating - the
    /// deliberate disagreement with the `Active` a record carrying no
    /// stored mode loads as (see `knot_core::SavedAgent::activation_mode`)
    /// - and the agent's own mode when editing.
    activation_mode:      knot_core::ActivationMode,
    /// Snapshot of `persona_id` when the dialog opened, so `save_edit` can
    /// tell `AgentStore::edit` whether the persona actually changed
    /// (`EditRequest::persona_changed`) rather than always forcing a
    /// restart.
    original_persona_id:  Option<Uuid>,
    /// What this editor was opened to derive the new agent from - carries
    /// the owner for a companion and the session for a fork, neither of
    /// which the form itself can express.
    prefill:              AgentPrefill,
    insert_after:         Option<Uuid>,
    /// `Some(id)` when editing an existing agent instead of creating one -
    /// gates prefill, the submit button's label/handler, and whether the
    /// (edit-unsupported) shell-command field shows at all.
    edit_target:          Option<Uuid>,
    on_created:           Box<AgentCreatedCallback>,
    error:                Option<String>,
}

pub(crate) type AgentCreatedCallback = dyn Fn(Uuid, &mut Window, &mut App);

/// What both submit paths take from the form once it is known to be
/// valid. Deliberately not the whole form: the fields each path reads
/// alone - a shell command when creating, a persona when editing - stay
/// where they are used.
struct AgentFields {
    folder: String,
    name:   String,
    avatar: String,
}

impl AgentEditor {
    /// Whether the form has everything required to submit - the primary
    /// button ("Add Agent" or "Save") is disabled until this is true.
    fn can_submit(&self, cx: &Context<Self>) -> bool {
        !self.name_input.read(cx).value().trim().is_empty()
        && !self.folder_path.trim().is_empty()
        && PathBuf::from(self.folder_path.trim()).is_dir()
    }

    /// The three fields both submit paths read, validated and trimmed, or
    /// `None` with `self.error` set and a repaint asked for.
    ///
    /// Create and edit validated these identically, in the same order,
    /// with the same two messages - so a rule changed in one was a rule
    /// changed in one. A caller's whole response to invalid input is now
    /// the `else` arm of a `let`.
    fn validated_fields(&mut self, cx: &mut Context<Self>) -> Option<AgentFields> {
        let folder = self.folder_path.trim().to_string();
        if folder.is_empty() || !PathBuf::from(&folder).is_dir() {
            self.error = Some(knot_core::l10n::t("agent_editor.error_choose_folder"));
            cx.notify();
            return None;
        }
        let name = self.name_input.read(cx).value().trim().to_string();
        if name.is_empty() {
            self.error = Some(knot_core::l10n::t("agent_editor.error_enter_name"));
            cx.notify();
            return None;
        }
        Some(AgentFields { folder,
                           name,
                           avatar: self.avatar_input.read(cx).value().trim().to_string() })
    }

    fn create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(AgentFields { folder,
                               name,
                               avatar, }) = self.validated_fields(cx)
        else {
            return;
        };
        // Defence in depth for the same invariant the form states above.
        let agent_type = created_agent_type(self.creating_a_companion(), &self.agent_type);
        let shell_command = self.shell_command_input.read(cx).value().trim().to_string();
        let description = self.description_input.read(cx).value().trim().to_string();
        let capabilities = parse_capability_tags(&self.capabilities_input.read(cx).value());
        let created_id = {
            let mut store = self.store.lock();
            let id = store.create(
                folder,
                knot_agents::CreateOptions {
                    name: Some(name),
                    avatar: (!avatar.is_empty()).then_some(avatar),
                    agent_type: (!agent_type.is_empty()).then_some(agent_type),
                    shell_command: (!shell_command.is_empty()).then_some(shell_command),
                    persona_id: self.persona_id,
                    insert_after: self.insert_after,
                    created_by: self.prefill.created_by,
                    is_companion: self.prefill.is_companion,
                    activation_mode: self.activation_mode,
                    workspace_id: Some(self.workspace_id),
                    description,
                    capabilities,
                    cost_tier: self.cost_tier,
                },
            );
            // A fork continues the source's conversation rather than
            // starting its own, so the new agent carries the session as
            // both its id and its resume target with the fork flag set -
            // `CreateOptions` has no field for any of the three, because
            // every other creation path starts a session from scratch.
            if let Some(session) = self.prefill.session_id.clone()
               && let Err(error) = store.fork_session(id, session)
            {
                eprintln!("failed to fork the agent's session: {error}");
            }
            self.settings.saved_agents =
                store.saved_agents(self.settings.restore_conversation_on_launch);
            self.settings.saved_workspaces = store.saved_workspaces();
            id
        };
        if let Err(error) = self.settings.persist() {
            eprintln!("failed to persist the agent roster: {error}");
        }
        (self.on_created)(created_id, window, cx);
        window.remove_window();
    }

    /// Applies edits to `self.edit_target` via `AgentStore::edit`. The
    /// shell-command field isn't submitted - `EditRequest` has no field for
    /// it, so the row is hidden in edit mode (see `Render for AgentEditor`).
    fn save_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.edit_target
        else {
            return;
        };
        let Some(AgentFields { folder,
                               name,
                               avatar, }) = self.validated_fields(cx)
        else {
            return;
        };
        let agent_type = self.agent_type.clone();
        let persona_changed = self.persona_id != self.original_persona_id;
        let description = self.description_input.read(cx).value().trim().to_string();
        let capabilities = parse_capability_tags(&self.capabilities_input.read(cx).value());
        {
            let mut store = self.store.lock();
            let result = store.edit(
                id,
                knot_agents::EditRequest {
                    name,
                    avatar,
                    folder: Some(folder),
                    agent_type: (!agent_type.is_empty()).then_some(agent_type),
                    persona_id: self.persona_id,
                    persona_changed,
                    relocate_companions: false,
                    activation_mode: self.activation_mode,
                    description,
                    capabilities,
                    cost_tier: self.cost_tier,
                },
            );
            if let Err(error) = result {
                self.error = Some(error.to_string());
                cx.notify();
                return;
            }
            self.settings.saved_agents =
                store.saved_agents(self.settings.restore_conversation_on_launch);
            self.settings.saved_workspaces = store.saved_workspaces();
        }
        if let Err(error) = self.settings.persist() {
            eprintln!("failed to persist the agent roster: {error}");
        }
        (self.on_created)(id, window, cx);
        window.remove_window();
    }

    /// Whether this editor is creating a companion rather than a standalone
    /// agent - true only in create mode, since editing never turns an agent
    /// into a companion.
    fn creating_a_companion(&self) -> bool {
        self.edit_target.is_none() && self.prefill.is_companion
    }

    fn choose_folder(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(knot_core::l10n::t("agent_editor.choose_folder_prompt").into()),
        });
        let editor = cx.entity();
        cx.spawn(async move |_this, cx| {
              let Ok(Ok(Some(paths))) = receiver.await
              else {
                  return;
              };
              let Some(path) = paths.into_iter().next()
              else {
                  return;
              };
              cx.update(|app| {
                    editor.update(app, |editor, cx| {
                              editor.folder_path = path.to_string_lossy().into_owned();
                              cx.notify();
                          });
                });
          })
          .detach();
    }

    /// Clears the avatar field, then focuses it and opens the OS character
    /// picker - it inserts the chosen character into whatever field has
    /// keyboard focus, so clearing first makes the picker replace the
    /// current avatar rather than append to it.
    fn choose_avatar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.avatar_input.update(cx, |input, cx| {
                             input.set_value("", window, cx);
                             input.focus(window, cx);
                         });
        #[cfg(target_os = "macos")]
        native_character_picker::open();
    }

    /// Keeps the avatar field to a single character (grapheme cluster), so
    /// typing or pasting past one character doesn't silently grow it.
    fn clamp_avatar_to_one_character(&mut self, avatar_input: &Entity<InputState>,
                                     window: &mut Window, cx: &mut Context<Self>) {
        let value = avatar_input.read(cx).value().to_string();
        let Some(first) = value.graphemes(true).next()
        else {
            return;
        };
        if first.len() == value.len() {
            return;
        }
        let first = first.to_string();
        avatar_input.update(cx, |input, cx| {
                        input.set_value(first, window, cx);
                    });
    }
}
