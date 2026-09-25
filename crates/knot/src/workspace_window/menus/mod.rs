//! The workspace window's two context menus.
//!
//! Both follow the same shape: a `*Targets` struct carrying what a handler
//! needs to act (the store, the window, the workspace), a `*_facts` reader
//! that snapshots the state the menu's contents depend on, a builder, and a
//! runner. The decision of *which entries exist and which are enabled* is
//! not here - it lives in [`crate::app_state`], where it is pure and tested.

mod agent_row;
mod menu_bar;
mod sidebar;

pub(crate) use agent_row::*;
use gpui_kit::App;
use gpui_kit::SharedString;
use gpui_kit::Window;
use gpui_kit::component::WindowExt;
pub(in crate::workspace_window) use menu_bar::*;
pub(crate) use sidebar::*;

/// Asks the user to confirm, and runs `on_confirm` if they do.
///
/// Deferred rather than opened inline because a `PopupMenu` dismisses
/// itself after running a handler and takes an inline dialog down with it -
/// which is why both menus' confirming items were written this way, four
/// times over, differing only in the two strings and the action.
pub(in crate::workspace_window) fn confirm_then(window: &mut Window, app: &mut App,
                                                title: impl Into<SharedString>,
                                                description: impl Into<SharedString>,
                                                on_confirm: impl Fn(&mut App) + Clone + 'static) {
    let title = title.into();
    let description = description.into();
    window.defer(app, move |window, app| {
              window.open_alert_dialog(app, move |alert, _, _| {
                        let on_confirm = on_confirm.clone();
                        alert.title(title.clone())
                             .description(description.clone())
                             .confirm()
                             .on_ok(move |_, _, app| {
                                 on_confirm(app);
                                 true
                             })
                    });
          });
}
