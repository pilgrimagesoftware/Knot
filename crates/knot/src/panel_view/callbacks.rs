//! The panel's interaction callbacks - what a row, a message or the
//! permission prompt calls back into when the user acts on it.

use std::rc::Rc;

use gpui_kit::App;
use gpui_kit::Window;
use knot_acp::PermissionDecision;
use uuid::Uuid;

/// Quotes one finished response into the composer. Takes the window because
/// it has to focus the composer.
pub(crate) type ReplyCallback = Rc<dyn Fn(String, &mut Window, &mut App)>;

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
    /// Stops one running `!` command, by its card id.
    pub(crate) on_cancel_shell:        Rc<dyn Fn(Uuid)>,
    /// Drops one finished `!` command's pending result, by its card id.
    pub(crate) on_discard_shell:       Rc<dyn Fn(Uuid)>,
    /// Quotes one finished response into the composer to reply to it.
    pub(crate) on_reply:               ReplyCallback,
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
               on_manual_scroll:       Rc::new(on_manual_scroll),
               on_cancel_shell:        Rc::new(|_| {}),
               on_discard_shell:       Rc::new(|_| {}),
               on_reply:               Rc::new(|_, _, _| {}), }
    }

    /// Adds the `!` command controls.
    ///
    /// Chained rather than two more parameters on [`Self::new`], which is
    /// already at the workspace's limit on argument count, and defaulted to
    /// no-ops so a surface that renders a conversation without owning the
    /// runs - a test, a future read-only view - does not have to invent them.
    pub(crate) fn with_shell(mut self, on_cancel: impl Fn(Uuid) + 'static,
                             on_discard: impl Fn(Uuid) + 'static)
                             -> Self {
        self.on_cancel_shell = Rc::new(on_cancel);
        self.on_discard_shell = Rc::new(on_discard);
        self
    }

    /// Adds the response action bar's Reply control (#559).
    ///
    /// Chained for the same reason as [`Self::with_shell`], and a no-op by
    /// default: a surface with no composer has nowhere to put the quote.
    pub(crate) fn with_reply(mut self,
                             on_reply: impl Fn(String, &mut Window, &mut App) + 'static)
                             -> Self {
        self.on_reply = Rc::new(on_reply);
        self
    }
}
