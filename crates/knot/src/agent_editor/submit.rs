//! What submitting the editor does: whether it may be submitted at all, what
//! it takes from the form when it is, and the two paths out - creating an
//! agent or saving an edit to one.

use std::path::PathBuf;

use gpui_kit::Context;
use gpui_kit::Window;

use super::created_agent_type;
use super::fields::AgentFields;
use super::parse_capability_tags;
use super::window::AgentEditor;

impl AgentEditor {
    /// Whether the form has everything required to submit - the primary
    /// button ("Add Agent" or "Save") is disabled until this is true.
    pub(super) fn can_submit(&self, cx: &Context<Self>) -> bool {
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

    pub(super) fn create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
            id
        };
        let installed = crate::settings_global::write(cx, |settings| {
            let store = self.store.lock();
            settings.saved_agents = store.saved_agents(settings.restore_conversation_on_launch);
            settings.saved_workspaces = store.saved_workspaces();
        });
        if let Err(error) = installed.persist_roster() {
            eprintln!("failed to persist the agent roster: {error}");
        }
        (self.on_created)(created_id, window, cx);
        window.remove_window();
    }

    /// Applies edits to `self.edit_target` via `AgentStore::edit`. The
    /// shell-command field isn't submitted - `EditRequest` has no field for
    /// it, so the row is hidden in edit mode (see `Render for AgentEditor`).
    pub(super) fn save_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
        }
        let installed = crate::settings_global::write(cx, |settings| {
            let store = self.store.lock();
            settings.saved_agents = store.saved_agents(settings.restore_conversation_on_launch);
            settings.saved_workspaces = store.saved_workspaces();
        });
        if let Err(error) = installed.persist_roster() {
            eprintln!("failed to persist the agent roster: {error}");
        }
        (self.on_created)(id, window, cx);
        window.remove_window();
    }

    /// Whether this editor is creating a companion rather than a standalone
    /// agent - true only in create mode, since editing never turns an agent
    /// into a companion.
    pub(super) fn creating_a_companion(&self) -> bool {
        self.edit_target.is_none() && self.prefill.is_companion
    }
}
