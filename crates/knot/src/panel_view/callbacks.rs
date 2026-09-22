//! The panel's interaction callbacks - what a row, a message or the
//! permission prompt calls back into when the user acts on it.

use std::rc::Rc;

use knot_acp::PermissionDecision;

/// The panel's interaction callbacks, grouped rather than threaded through
/// the row renderer and message renderer as four separate parameters. Wrapped
/// in `Rc` so the list's row closure can hand cheap clones to each row it
/// materializes; `PanelState` remains the only thing shared with the ACP
/// reader thread, so this stays on the UI thread.
#[derive(Clone)]
pub(crate) struct PanelCallbacks {
    pub(crate) on_permission_decision: Rc<dyn Fn(PermissionDecision)>,
    pub(crate) on_toggle_track:        Rc<dyn Fn()>,
    pub(crate) on_toggle_tool_call:    Rc<dyn Fn(String)>,
    /// Opens or closes one compact summary's run, keyed by the id of its
    /// first tool call.
    pub(crate) on_toggle_tool_run:     Rc<dyn Fn(String)>,
    pub(crate) on_manual_scroll:       Rc<dyn Fn()>,
}

impl PanelCallbacks {
    pub(crate) fn new(on_permission_decision: impl Fn(PermissionDecision) + 'static,
                      on_toggle_track: impl Fn() + 'static,
                      on_toggle_tool_call: impl Fn(String) + 'static,
                      on_toggle_tool_run: impl Fn(String) + 'static,
                      on_manual_scroll: impl Fn() + 'static)
                      -> Self {
        Self { on_permission_decision: Rc::new(on_permission_decision),
               on_toggle_track:        Rc::new(on_toggle_track),
               on_toggle_tool_call:    Rc::new(on_toggle_tool_call),
               on_toggle_tool_run:     Rc::new(on_toggle_tool_run),
               on_manual_scroll:       Rc::new(on_manual_scroll), }
    }
}
