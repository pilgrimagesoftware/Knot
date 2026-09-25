//! Which window is open for what, so a second request to open something
//! raises the window that already exists instead of making another.
//!
//! Three windows - About, Settings, Import - each solved this for themselves
//! with an `Rc<RefCell<Option<AnyWindowHandle>>>` captured by the closure that
//! registers their action. That works for a window there is only ever one of.
//! A workspace window needs a map rather than a slot, and the routes that open
//! one (`workspace_manager`, `command_center`, the agent editor's post-create
//! jump) belong to no single owner that could hold it, so the map lives here as
//! a global.
//!
//! Liveness is asked by updating the handle: a closed window fails its update.
//! That is the same test the three existing singletons use, and it is why no
//! close observer is needed - a stale entry is noticed and dropped on the next
//! request for its key.
//!
//! Contract: `openspec/specs/window-lifecycle/spec.md`.

use std::collections::HashMap;

use gpui_kit::AnyWindowHandle;
use gpui_kit::App;
use gpui_kit::Entity;
use gpui_kit::WeakEntity;
use uuid::Uuid;

use crate::workspace_window::WorkspaceWindow;

/// What a window is open *for*, which is what a repeat request names.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum WindowKey {
    Workspace(Uuid),
    CommandCenter,
    WorkspaceManager,
}

/// One open window.
///
/// `workspace` is the window's `WorkspaceWindow` entity, held weakly so a
/// registry entry never keeps a closed window's view alive. It is `None` for
/// the Command Center and the workspace manager, which have no per-window
/// state a caller needs to reach on the raise path.
///
/// The entity is kept rather than recovered from the handle because
/// `cx.open_window` hands back a `WindowHandle<Root>` - the root entity is the
/// `Root` wrapper, not the view inside it - so `AnyWindowHandle::downcast` to
/// `WorkspaceWindow` never matches.
struct Registered {
    handle:    AnyWindowHandle,
    workspace: Option<WeakEntity<WorkspaceWindow>>,
}

/// The open windows, keyed by what they are open for.
#[derive(Default)]
pub(crate) struct WindowRegistry {
    windows: HashMap<WindowKey, Registered>,
}

impl gpui_kit::Global for WindowRegistry {}

impl WindowRegistry {
    /// Installs the registry, if it is not installed already.
    ///
    /// Every operation below is total on a missing global, so this is only
    /// needed to make the first `register` land somewhere. Tests that open
    /// windows without going through `run` get the same behaviour either way.
    pub(crate) fn install(cx: &mut App) {
        if !cx.has_global::<Self>() {
            cx.set_global(Self::default());
        }
    }

    /// Raises the window open for `key`, reporting whether there was one.
    ///
    /// A handle whose update fails belongs to a window that has since closed;
    /// its entry is dropped here, which is what keeps `Workspace` entries from
    /// accumulating across a long session.
    pub(crate) fn activate(key: WindowKey, cx: &mut App) -> bool {
        let Some(handle) = Self::handle(key, cx)
        else {
            return false;
        };
        if handle.update(cx, |_, window, _| window.activate_window())
                 .is_ok()
        {
            return true;
        }
        Self::forget(key, cx);
        false
    }

    /// The workspace window open for `key`, if one still is.
    ///
    /// Callers that also need to change what the raised window is showing - a
    /// Command Center card naming an agent - go through this after `activate`.
    pub(crate) fn workspace_view(key: WindowKey, cx: &mut App) -> Option<Entity<WorkspaceWindow>> {
        cx.try_global::<Self>()?
          .windows
          .get(&key)?
          .workspace
          .as_ref()?
          .upgrade()
    }

    /// Records a window that has just opened.
    ///
    /// `workspace` is the view for a workspace window and `None` for the
    /// others. Registering over an existing key replaces it, which is what
    /// makes a re-open after a close land on the new window rather than the
    /// handle of the old one.
    pub(crate) fn register(key: WindowKey, handle: AnyWindowHandle,
                           workspace: Option<WeakEntity<WorkspaceWindow>>, cx: &mut App) {
        Self::install(cx);
        cx.global_mut::<Self>()
          .windows
          .insert(key, Registered { handle, workspace });
    }

    /// Drops `key`'s entry, so the next request opens a window.
    pub(crate) fn forget(key: WindowKey, cx: &mut App) {
        if cx.has_global::<Self>() {
            cx.global_mut::<Self>().windows.remove(&key);
        }
    }

    /// `key`'s handle, without asking whether its window is still open.
    fn handle(key: WindowKey, cx: &App) -> Option<AnyWindowHandle> {
        Some(cx.try_global::<Self>()?.windows.get(&key)?.handle)
    }
}

/// Raises the window open for `key`, or opens one with `open` and records it,
/// reporting whether a window was opened.
///
/// `open` reports the new window's handle, or `None` when the open failed - in
/// which case nothing is recorded and the next request tries again.
///
/// For windows with no per-window state a caller reaches on the raise path,
/// which is every one but a workspace window. A workspace window registers
/// itself from inside its open closure, where its view is in scope.
pub(crate) fn activate_or_open<F>(key: WindowKey, cx: &mut App, open: F) -> bool
    where F: FnOnce(&mut App) -> Option<AnyWindowHandle> {
    if WindowRegistry::activate(key, cx) {
        return false;
    }
    if let Some(handle) = open(cx) {
        WindowRegistry::register(key, handle, None, cx);
    }
    true
}
