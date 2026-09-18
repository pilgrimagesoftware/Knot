#![allow(dead_code)]

mod agent_editor;
mod app_bootstrap;
mod app_state;
mod app_support;
mod command_center;
mod dashboard;
mod panel_session;
mod panel_state;
mod panel_view;
mod settings_window;
mod terminal_view;
#[cfg(test)]
mod tests;
mod window_options;
mod workspace_manager;
mod workspace_window;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
#[cfg(target_os = "macos")]
use std::time::Duration;

use agent_editor::*;
use app_state::*;
use app_support::*;
use command_center::*;
use gpui_kit::base::Selectable;
use gpui_kit::base::{h_flex, v_flex};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::group_box::{GroupBox, GroupBoxVariants};
use gpui_kit::component::input::{Input, InputEvent, InputState, Paste, Textarea, TextareaState};
use gpui_kit::component::menu::{ContextMenuExt, DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::switch::Switch;
use gpui_kit::component::tab::{Tab, TabBar};
use gpui_kit::component::*;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    AnyWindowHandle, App, AppContext, ClickEvent, ClipboardEntry, ClipboardItem, Context, Entity,
    ImageFormat, InteractiveElement, IntoElement, KeyBinding, Menu, MenuItem, ParentElement,
    PathPromptOptions, Render, StatefulInteractiveElement, Styled, Subscription, SystemMenuType,
    SystemNotificationResponse, WeakEntity, Window, WindowBounds, WindowOptions, actions, div, px,
    rgb, size,
};
use knot_activity::EventSink;
use knot_agent_launch::acp_adapter;
use knot_git::Repository;
use knot_mcp::ToolCatalog;
use knot_messaging::{DeliveryEvent, QueuedNotifier};
use knot_terminal::{PtyTransport, SessionConfig, SessionPlan, TerminalSession};
use settings_window::*;
use unicode_segmentation::UnicodeSegmentation;
use uuid::Uuid;
use window_options::*;
use workspace_manager::*;
use workspace_window::*;

fn main() {
    app_bootstrap::run();
}
