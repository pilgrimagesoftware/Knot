//! The keys the sidebar shows while ⌘ is held: which label goes on which
//! control, when they appear and disappear, and the badge they are drawn in.
//!
//! Contract: `openspec/specs/agent-list-ui/spec.md` ("Holding ⌘ shows the
//! sidebar's shortcuts"). Design: `openspec/changes/sidebar-key-hints/`.
//!
//! The labels are formatted once, when a hold turns the hints on, and kept
//! until it ends. The render path only reads them. A rebinding cannot land
//! mid-hold: recording one needs the settings window focused, and leaving
//! this window ends the hold.

use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::KeyDownEvent;
use gpui_kit::ModifiersChangedEvent;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::component::ActiveTheme;
use gpui_kit::div;
use gpui_kit::px;

use crate::consts;
use crate::keymap::Chord;
use crate::keymap::Resolved;
use crate::keymap::Shortcut;
use crate::workspace_window::WorkspaceWindow;

/// What each sidebar control's hint reads, in the glyph form macOS menus use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SidebarKeyHints {
    pub(crate) dashboard:     String,
    pub(crate) pull_requests: String,
    pub(crate) new_agent:     String,
    /// Index N is the Nth agent row's; rows past the ninth have none.
    pub(crate) agents:        Vec<String>,
}

impl SidebarKeyHints {
    pub(crate) fn from_resolved(resolved: &Resolved) -> Self {
        let single = |shortcut| {
            resolved.chord(shortcut)
                    .map(Chord::label)
                    .unwrap_or_default()
        };
        Self { dashboard:     single(Shortcut::ToggleDashboard),
               pull_requests: single(Shortcut::TogglePullRequests),
               new_agent:     Chord::parse(consts::NEW_AGENT_CHORD).map(|chord| chord.label())
                                                                   .unwrap_or_default(),
               agents:        resolved.chords_of(Shortcut::SelectAgent)
                                      .iter()
                                      .map(Chord::label)
                                      .collect(), }
    }

    /// The Nth agent row's hint, `None` past the ninth.
    pub(crate) fn agent(&self, index: usize) -> Option<&str> {
        self.agents.get(index).map(String::as_str)
    }
}

/// Where a ⌘ hold is: not held, held but not yet long enough, or showing.
#[derive(Debug, Default)]
pub(crate) struct KeyHintHold {
    /// Bumped by every change, so a delay started for an earlier hold can
    /// tell it is stale when it fires.
    generation: u64,
    armed:      bool,
    shown:      Option<SidebarKeyHints>,
}

impl KeyHintHold {
    pub(crate) fn shown(&self) -> Option<&SidebarKeyHints> {
        self.shown.as_ref()
    }

    /// Ends any hold. Whether anything was showing, which is when a repaint
    /// is owed.
    fn clear(&mut self) -> bool {
        self.generation += 1;
        self.armed = false;
        self.shown.take().is_some()
    }
}

impl WorkspaceWindow {
    /// ⌘ going down starts the delay; ⌘ coming up ends the hold. Other
    /// modifiers do neither, so adding ⌥ for an agent's key keeps the hints.
    pub(crate) fn key_hints_modifiers_changed(&mut self, event: &ModifiersChangedEvent,
                                              cx: &mut Context<Self>) {
        let command = event.modifiers.platform;
        if !command {
            if self.key_hints.clear() {
                cx.notify();
            }
            return;
        }
        if self.key_hints.armed {
            return;
        }
        self.key_hints.generation += 1;
        self.key_hints.armed = true;
        let generation = self.key_hints.generation;
        cx.spawn(async move |view, cx| {
              cx.background_executor().timer(consts::KEY_HINT_DELAY).await;
              view.update(cx, |view, cx| view.show_key_hints(generation, cx))
                  .ok();
          })
          .detach();
    }

    /// A key pressed during the hold ends it: the user was typing a shortcut,
    /// not asking what the shortcuts are.
    pub(crate) fn key_hints_key_down(&mut self, _: &KeyDownEvent, cx: &mut Context<Self>) {
        if self.key_hints.clear() {
            cx.notify();
        }
    }

    /// Ends the hold once the window stops being the key window. macOS does
    /// not deliver the ⌘ key-up to a window that lost key status on ⌘Tab.
    pub(crate) fn key_hints_follow_activation(&mut self, window: &Window) {
        if !window.is_window_active() {
            self.key_hints.clear();
        }
    }

    fn show_key_hints(&mut self, generation: u64, cx: &mut Context<Self>) {
        if !self.key_hints.armed || self.key_hints.generation != generation {
            return;
        }
        let resolved = Resolved::from_settings(&crate::settings_global::read(cx).keybindings);
        self.key_hints.shown = Some(SidebarKeyHints::from_resolved(&resolved));
        cx.notify();
    }
}

/// Adds `hint`'s badge to `el`, drawn over it without taking any of its
/// space - which is what keeps rows from moving when the hints appear.
///
/// `compact` puts it over the corner of `el`, which is then the avatar or
/// icon; otherwise it sits against `el`'s trailing edge, centred, over the
/// row's own trailing content.
pub(crate) fn with_key_hint<E: ParentElement + Styled>(el: E, hint: Option<&str>, compact: bool,
                                                       cx: &gpui_kit::App)
                                                       -> E {
    let Some(label) = hint
    else {
        return el;
    };
    let badge = div().debug_selector(|| "key-hint".into())
                     .px_1()
                     .rounded(px(4.))
                     .bg(cx.theme().secondary)
                     .border_1()
                     .border_color(cx.theme().border)
                     .text_xs()
                     .text_color(cx.theme().secondary_foreground)
                     .whitespace_nowrap()
                     .child(label.to_string());
    let placed = if compact {
        div().absolute().top(px(-6.)).right(px(-10.)).child(badge)
    }
    else {
        div().absolute()
             .top_0()
             .bottom_0()
             .right_2()
             .flex()
             .items_center()
             .child(badge)
    };
    el.relative().child(placed)
}
