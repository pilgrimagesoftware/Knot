//! What the menu bar was last built from, and which window it belongs to.
//!
//! Contract: `openspec/specs/app-menu/spec.md`.
//!
//! A [`gpui_kit::Menu`] is a static snapshot: submenu contents, checkmarks
//! and a submenu parent's enabled state do not re-evaluate on their own, so
//! the bar is rebuilt whenever the facts behind them change. This is the
//! record those rebuilds compare against.

use crate::agent_menu::AgentMenuSnapshot;
use crate::view_menu::ViewMenuSnapshot;

/// Everything the menu bar's dynamic parts are built from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct MenuBarSnapshot {
    pub(crate) agents: AgentMenuSnapshot,
    pub(crate) view:   ViewMenuSnapshot,
}

/// Which window's state the menu bar is currently showing, and what it was
/// built from.
///
/// The menu bar is app-wide but the selection it acts on belongs to one
/// window, so ownership has to be recorded somewhere both windows can see:
/// the active window claims it, and only the window that holds it may give
/// it up. Without that, two open workspace windows would overwrite each
/// other's submenus on alternating polls.
#[derive(Default)]
pub(crate) struct MenuBarState {
    pub(crate) owner:    Option<gpui_kit::AnyWindowHandle>,
    pub(crate) snapshot: MenuBarSnapshot,
}

impl gpui_kit::Global for MenuBarState {}
