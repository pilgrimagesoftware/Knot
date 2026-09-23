//! What the workspace manager window knows.
//!
//! Split from the methods that change it (`actions`), the dialog that edits
//! a name (`dialog`) and what draws it (`render`), so the window's state can
//! be read without reading its behaviour.

use std::sync::Arc;

use gpui_kit::Entity;
use gpui_kit::Subscription;
use gpui_kit::component::input::InputState;
use parking_lot::Mutex;
use uuid::Uuid;

/// Whether a typed workspace name counts as nothing at all.
///
/// The confirm button's disabled state and the Return key both read this,
/// so a name the button refuses is a name Return refuses, by construction
/// rather than by two guards kept in step by hand. Judged on the trimmed
/// form, which is also the form `save_name` stores.
pub(crate) fn workspace_name_is_blank(name: &str) -> bool {
    name.trim().is_empty()
}

pub(crate) struct WorkspaceManager {
    pub(crate) store:               Arc<Mutex<knot_agents::AgentStore>>,
    pub(crate) messages:            Arc<Mutex<knot_messaging::MessageStore>>,
    pub(crate) name_input:          Entity<InputState>,
    /// The workspace the open name dialog is renaming, or `None` when it is
    /// naming a new one. Whether a dialog is open at all is the `Root`'s
    /// answer, not a flag here - see [`crate::workspace_manager::dialog`].
    pub(crate) workspace_dialog_id: Option<Uuid>,
    pub(crate) error:               Option<String>,
    /// Keeps the confirm button's disabled state honest while the user
    /// types: without it the button only re-reads the name on the next
    /// unrelated re-render, so an empty field could stay greyed after the
    /// first character.
    pub(crate) _name_subscription:  Subscription,
    pub(crate) _mcp_stop:           Option<tokio::sync::oneshot::Sender<()>>,
}

#[derive(Clone)]
pub(crate) struct WorkspaceDrag(pub(crate) Uuid);

pub(crate) struct WorkspaceDragPreview;
