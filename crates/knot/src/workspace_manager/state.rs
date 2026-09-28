//! What the workspace manager window knows.
//!
//! Split from the methods that change it (`actions`), the dialog that edits
//! a name (`dialog`) and what draws it (`render`), so the window's state can
//! be read without reading its behaviour.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gpui_kit::Bounds;
use gpui_kit::Entity;
use gpui_kit::Pixels;
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
    /// Where a dragged row would land, and where each row was last painted.
    pub(crate) drag:                RowDrag,
}

/// A drag-to-reorder in progress, as far as the manager needs to know it.
#[derive(Default)]
pub(crate) struct RowDrag {
    /// The insertion gap under the cursor, and which row reported it. The
    /// row is kept so a row the cursor leaves clears only its own answer -
    /// every row hears every move, and the one it is over speaks last or
    /// first depending on paint order.
    pub(crate) target:     Option<DropTarget>,
    /// Each row's bounds from the last frame, in row order, so the drag
    /// preview can be laid over the row it was lifted from. Written during
    /// prepaint, read when a drag starts - both on the main thread.
    pub(crate) row_bounds: Rc<RefCell<Vec<Bounds<Pixels>>>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DropTarget {
    pub(crate) row: usize,
    /// Counted in the current order: `n` is above row `n`, `len` below the
    /// last. What [`knot_agents::AgentStore::move_workspace_to_gap`] takes.
    pub(crate) gap: usize,
}
