//! The name a workspace window shows for itself - both in the title bar it
//! draws and in the OS window title the Window menu, Cmd+` and Mission
//! Control read.
//!
//! Contract: `openspec/specs/agent-list-ui/spec.md` - "The sidebar's title
//! bar names the workspace".
//!
//! A free function over the store rather than a method on the window, so the
//! whole decision is testable without standing up a window, and so both
//! places that need the name resolve it the same way instead of agreeing by
//! coincidence.

use uuid::Uuid;

use crate::app_support::single_line;

/// The title to show for the workspace `id` names, or `None` when the store
/// holds no such workspace.
///
/// `None` deliberately carries no fallback string: the only caller that can
/// see it is a window whose workspace was deleted, and that window returns
/// early with `workspace.missing` and an empty title bar before it ever
/// draws a name.
///
/// Flattened through [`single_line`] because the title bar declares one line
/// and a workspace name is user data - `whitespace_nowrap` alone would draw
/// an embedded newline as a second row and push the traffic lights down.
pub(crate) fn workspace_title(store: &knot_agents::AgentStore, id: Uuid) -> Option<String> {
    store.workspaces()
         .iter()
         .find(|workspace| workspace.id == id)
         .map(|workspace| single_line(&workspace.name))
}
