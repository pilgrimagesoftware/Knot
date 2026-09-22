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
pub(in crate::workspace_window) use menu_bar::*;
pub(crate) use sidebar::*;
