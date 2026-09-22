//! The Import window's state and lifecycle: what it scanned, what the user
//! ticked, what the last import did, and how the window opens.
//!
//! Contract: `openspec/specs/import-ui/spec.md`, over
//! `openspec/specs/data-import/spec.md`.
//!
//! An import is something the user runs, not something they configure, so it
//! is a window of its own reached from File rather than a settings tab. A
//! single instance, like About and Settings - choosing Import again raises
//! the one that is open.
//!
//! Rendering lives in [`super::pane`]; this file holds only state and the
//! actions that change it.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use gpui_kit::AnyWindowHandle;
use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Root;
use knot_core::import::{SkwadSource, SubagentScan, SubagentTool};
use uuid::Uuid;

use crate::app_support::observe_system_appearance;
use crate::window_options::import_window_options;

/// What the window found when it last looked.
///
/// Held rather than re-read per frame: GPUI re-renders on every keystroke,
/// and two directory walks plus a plist parse per frame is exactly the I/O
/// the render path must not do.
#[derive(Default)]
pub(crate) struct ImportSources {
    pub(crate) subagents: SubagentScan,
    pub(crate) skwad:     SkwadSource,
}

pub(crate) struct ImportWindow {
    /// Focused on first render so the window has a key target for `Escape`.
    /// A window with nothing focused never sees the key event at all.
    pub(super) focus:               gpui_kit::FocusHandle,
    /// The store an import writes into. Loaded when the window opens rather
    /// than snapshotted at bootstrap, for the reason the settings window
    /// reloads: importing into a stale copy would overwrite whatever has
    /// been saved since.
    pub(super) settings:            knot_core::Settings,
    pub(super) sources:             ImportSources,
    pub(super) subagent_selection:  BTreeSet<String>,
    pub(super) workspace_selection: BTreeSet<Uuid>,
    pub(super) result:              Option<knot_core::import::ImportResult>,
}

impl ImportWindow {
    fn new(settings: knot_core::Settings, cx: &mut Context<Self>) -> Self {
        Self { focus: cx.focus_handle(),
               settings,
               sources: Self::scan(),
               subagent_selection: BTreeSet::new(),
               workspace_selection: BTreeSet::new(),
               result: None }
    }

    /// Read every source. Called when the window opens and when the user
    /// asks it to look again - never from `render`.
    fn scan() -> ImportSources {
        let subagents = SubagentTool::implemented().into_iter()
                                                   .filter_map(knot_core::import::provider)
                                                   .map(|provider| provider.definitions(None))
                                                   .fold(SubagentScan::default(), merge_scans);
        ImportSources { subagents,
                        skwad: knot_core::import::skwad::read() }
    }

    /// Re-read every source, discarding the cache and any pending selection -
    /// a tick against a definition that is no longer there would import
    /// nothing and say nothing.
    pub(super) fn refresh(&mut self) {
        self.sources = Self::scan();
        self.subagent_selection.clear();
        self.workspace_selection.clear();
        self.result = None;
    }

    pub(super) fn import_selected_definitions(&mut self, cx: &mut Context<Self>) {
        let selected: Vec<_> = self.sources
                                   .subagents
                                   .definitions
                                   .iter()
                                   .filter(|d| self.subagent_selection.contains(&d.source))
                                   .cloned()
                                   .collect();

        match knot_core::import::import_definitions(&mut self.settings, &selected) {
            Ok(mut result) => {
                // What the scan itself could not read belongs in the same
                // summary: the user asked to import from a source, and a file
                // that never made the list is part of that answer.
                result.unreadable
                      .extend(self.sources.subagents.unreadable.clone());
                self.result = Some(result);
            }
            Err(error) => eprintln!("failed to import subagent definitions: {error}"),
        }
        self.subagent_selection.clear();
        cx.notify();
    }

    pub(super) fn import_selected_workspaces(&mut self, cx: &mut Context<Self>) {
        let source = self.sources.skwad.clone();
        let selected: Vec<Uuid> = self.workspace_selection.iter().copied().collect();

        match knot_core::import::import_workspaces(&mut self.settings, &source, &selected) {
            Ok(mut result) => {
                result.unreadable.extend(source.unreadable.clone());
                self.result = Some(result);
            }
            Err(error) => eprintln!("failed to import Skwad workspaces: {error}"),
        }
        self.workspace_selection.clear();
        cx.notify();
    }
}

impl Render for ImportWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Focusing here rather than at open time: the focus handle only has
        // an element to attach to once this tree exists.
        if window.focused(cx).is_none() {
            window.focus(&self.focus.clone(), cx);
        }

        v_flex().track_focus(&self.focus)
                .on_key_down(cx.listener(|_, event: &gpui_kit::KeyDownEvent, window, _| {
                                   // Scoped to this window's own focus handle
                                   // rather than an
                                   // app-wide `Escape` binding, which would
                                   // take the key away
                                   // from every other window.
                                   if event.keystroke.key == "escape" {
                                       window.remove_window();
                                   }
                               }))
                .size_full()
                .gap_3()
                .p_4()
                .bg(cx.theme().background)
                .child(self.render_import(cx))
                .children(crate::app_support::root_overlays(window, cx))
    }
}

/// Registers the `OpenImport` handler over the window handle it keeps.
///
/// The handle lives in this closure rather than in `run`, so nothing outside
/// can open a second Import window past the single-instance check. `run` and
/// the window tests both register through here, so the tests exercise the
/// wiring the app actually installs.
pub(crate) fn register_import_action(cx: &mut App) {
    let handle: Rc<RefCell<Option<AnyWindowHandle>>> = Rc::new(RefCell::new(None));
    cx.on_action(move |_: &crate::app_bootstrap::OpenImport, cx| {
          // Reload from disk rather than reusing a clone captured at
          // bootstrap: an import writes to the store, and importing into a
          // stale snapshot would overwrite anything saved since.
          let settings = knot_core::Settings::load().unwrap_or_default();
          open_import_window(&handle, settings, cx);
      });
}

/// Raises the open Import window, or opens one.
///
/// Whether the last window is still open is asked by updating it: a closed
/// window fails its update, which is the same test the settings and About
/// windows use, and is why no close observer is needed to clear the handle.
pub(crate) fn open_import_window(handle: &Rc<RefCell<Option<AnyWindowHandle>>>,
                                 settings: knot_core::Settings, cx: &mut App) {
    if let Some(existing) = *handle.borrow()
       && existing.update(cx, |_, window, _| window.activate_window())
                  .is_ok()
    {
        return;
    }
    match cx.open_window(import_window_options(cx), move |window, cx| {
                // Every window tracks the OS appearance, so a light/dark flip
                // re-resolves the system palette and repaints.
                observe_system_appearance(window);
                let view = cx.new(|cx| ImportWindow::new(settings, cx));
                cx.new(|cx| Root::new(view, window, cx).bg(cx.theme().background))
            }) {
        Ok(window) => *handle.borrow_mut() = Some(window.into()),
        Err(error) => eprintln!("failed to open the Import window: {error}"),
    }
}

/// Fold one tool's scan into the running total, keeping definitions and
/// unreadable records in the order the tools were asked.
fn merge_scans(mut total: SubagentScan, next: SubagentScan) -> SubagentScan {
    total.definitions.extend(next.definitions);
    total.unreadable.extend(next.unreadable);
    total
}
