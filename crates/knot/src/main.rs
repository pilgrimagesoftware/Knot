//! The `knot` binary.
//!
//! NOTE: no crate-wide `allow(dead_code)`. Anything unreachable carries its
//! own `#[allow(dead_code)]` and a comment saying why - grep `UNWIRED` for
//! the ported-but-not-yet-connected inventory, and `SUPERSEDED` for code a
//! newer path replaced.

mod about_window;
mod agent_editor;
mod agent_menu;
mod app_bootstrap;
mod app_state;
mod app_support;
mod broadcast_sheet;
mod command_center;
mod consts;
mod dashboard;
mod macos;
mod open_in;
mod panel_session;
mod panel_state;
mod panel_view;
mod quit_guard;
mod settings_window;
mod terminal_view;
#[cfg(test)]
mod tests;
mod window_options;
mod working_indicator;
mod workspace_manager;
mod workspace_window;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use about_window::*;
use agent_editor::*;
use agent_menu::*;
use app_state::*;
use app_support::*;
use broadcast_sheet::*;
use command_center::*;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::group_box::{GroupBox, GroupBoxVariants};
use gpui_kit::component::input::{
    Escape, Input, InputEvent, InputState, Paste, Textarea, TextareaState,
};
use gpui_kit::component::menu::{ContextMenuExt, DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::text::TextView;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::*;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AnyWindowHandle, App, AppContext, ClickEvent, ClipboardEntry, ClipboardItem, Context, Entity,
    FollowMode, ImageFormat, InteractiveElement, IntoElement, KeyBinding, ListAlignment, ListState,
    Menu, MenuItem, ParentElement, PathBuilder, PathPromptOptions, Render,
    StatefulInteractiveElement, Styled, Subscription, SystemMenuType, SystemNotificationResponse,
    WeakEntity, Window, WindowBounds, WindowOptions, actions, canvas, div, point, px, rgb, size,
};
use knot_activity::EventSink;
use knot_git::Repository;
use knot_mcp::ToolCatalog;
use knot_messaging::{DeliveryEvent, QueuedNotifier};
use knot_terminal::{PtyTransport, SessionConfig, SessionPlan, TerminalSession};
use parking_lot::Mutex;
use settings_window::*;
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;
use window_options::*;
use workspace_manager::*;
use workspace_window::*;

fn main() {
    app_bootstrap::run();
}
