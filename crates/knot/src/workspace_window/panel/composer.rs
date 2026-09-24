//! The composer entity itself: how one is built, how long it lives, and
//! which chord sends it.
//!
//! Split from [`super::prompt`], which is about getting a prompt from the
//! composer into the agent - the queue, the delivery, the stop. This is the
//! widget side of that: the `EditorState` behind the prompt box, its
//! auto-grow bounds, its cache, and the `agent_panel_shift_enter_sends`
//! setting that decides what `Enter` and `Shift+Enter` each do.
//!
//! The send chord is the reason this is its own file rather than a section
//! of that one. It is split across three places that have to agree -
//! [`sends_now`] at the keypress, `submit_on_enter` inside the entity, and
//! the hint the user reads - and #429 was two of the three disagreeing.

use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::Window;
use gpui_kit::component::input::InputEvent;
use uuid::Uuid;

use super::prompt::PANEL_INPUT_ROWS_COLLAPSED;
use super::prompt::PANEL_INPUT_ROWS_EXPANDED;
use super::prompt::PanelInputState;
use crate::composer_style::Palette;
use crate::workspace_window::WorkspaceWindow;

#[cfg(test)]
mod tests;

/// How far `id`'s composer may grow, given whether it is expanded.
///
/// The bound is read in two places - when the entity is built and when the
/// expand control re-issues it - and they have to agree, so the choice
/// lives here rather than twice.
pub(crate) fn panel_input_max_rows(expanded: bool) -> usize {
    if expanded {
        PANEL_INPUT_ROWS_EXPANDED
    }
    else {
        PANEL_INPUT_ROWS_COLLAPSED
    }
}

/// Whether a `PressEnter` carrying `shift` is the send chord, given the
/// `agent_panel_shift_enter_sends` setting.
///
/// The widget reports *both* chords as `PressEnter` and distinguishes them
/// only by this flag - `submit_on_enter` changes which one also types a
/// newline, not which one reports - so this predicate is the whole of the
/// send-chord decision.
pub(crate) fn sends_on(shift_to_send: bool, shift: bool) -> bool {
    shift == shift_to_send
}

/// [`sends_on`] against the setting *as it stands now*.
///
/// The composer entity and its subscription outlive any number of settings
/// changes - `panel_prompt_input` builds them once per agent and hands the
/// same pair back on every later call - so the decision has to read the
/// shared surface when the chord is pressed. Capturing the flag into the
/// subscription instead is #429: the hint below the prompt box and the Send
/// button tooltip read live and moved, and the keys kept doing what they did
/// when the composer was built.
pub(crate) fn sends_now(shift: bool, cx: &App) -> bool {
    sends_on(crate::settings_global::read(cx).agent_panel_shift_enter_sends,
             shift)
}

/// Builds the composer's state entity: the placeholder, the send chord
/// implied by `shift_to_send`, and the auto-grow bounds.
///
/// Separate from [`WorkspaceWindow::panel_prompt_input`], which owns the
/// caching and the event subscription, so the widget's own configuration
/// can be exercised in a test without a window's worth of state behind it.
pub(crate) fn new_panel_input(shift_to_send: bool, max_rows: usize, window: &mut Window,
                              cx: &mut App)
                              -> Entity<PanelInputState> {
    cx.new(|cx| {
          PanelInputState::new(window, cx).placeholder(WorkspaceWindow::panel_prompt_placeholder())
                                          // `submit_on_enter` is the
                                          // inverse of the setting: the
                                          // chord that does *not* send is
                                          // the one that inserts a newline.
                                          .submit_on_enter(!shift_to_send)
                                          // `EditorState::new` turns the
                                          // built-in search panel on. A
                                          // composer is written, not
                                          // searched, and off also lets
                                          // Cmd-F bubble to the window.
                                          .searchable(false)
                                          // Last, and load-bearing:
                                          // `EditorState::new` starts in
                                          // `LayoutMode::CodeEditor` and
                                          // this replaces the mode
                                          // outright. Line numbers, the
                                          // gutter, indent guides,
                                          // folding, auto-closing brackets
                                          // and smart indent are all
                                          // fields of that variant or
                                          // gated on `is_code_editor()`,
                                          // so leaving it is what makes
                                          // "the composer stays a
                                          // composer" true by
                                          // construction rather than by
                                          // turning flags off one at a
                                          // time.
                                          .auto_grow(1, max_rows)
      })
}

impl WorkspaceWindow {
    /// Toggles `id`'s input area between its default and expanded
    /// multi-line editing size.
    pub(in crate::workspace_window) fn toggle_panel_input_expanded(&mut self, id: Uuid,
                                                                   cx: &mut Context<Self>) {
        let expanded = if self.panel_input_expanded.remove(&id) {
            false
        }
        else {
            self.panel_input_expanded.insert(id);
            true
        };
        // The cap is part of the textarea's own layout mode, so expanding
        // has to update the live entity rather than just the render height.
        let max_rows = panel_input_max_rows(expanded);
        if let Some(input) = self.panel_prompt_inputs.get(&id).cloned() {
            cx.update_entity(&input, |state, cx| state.set_auto_grow(1, max_rows, cx));
        }
    }

    /// Brings every composer this window already built in line with the
    /// `agent_panel_shift_enter_sends` setting as it stands now.
    ///
    /// Only the widget's own `submit_on_enter` needs this. Which chord
    /// *sends* is [`sends_now`], read when the key is pressed; which chord
    /// types a newline is a flag inside the entity, set when it was built,
    /// and an entity built before the setting changed is still holding the
    /// old one. Left alone, the chord the hint now calls "newline" reaches
    /// `cx.propagate()` instead of inserting one, and whether the user gets
    /// their newline is then down to what else happens to handle `Enter`.
    ///
    /// `panel_input_send_chord` is what the composers were last set to, not
    /// a copy of the setting to read from: `set_submit_on_enter` notifies
    /// unconditionally, so re-applying an unchanged value would repaint
    /// forever. Returns whether anything was set, for the notify chain.
    ///
    /// Called from `repaint_poll_tick` rather than from a render. The writer
    /// is the settings window and `settings_global::write` notifies nobody,
    /// so a preference change reaches this window only when something asks -
    /// and a render is not something that is guaranteed to happen. On a
    /// quiet workspace the window may not redraw at all, which is the "could
    /// be never" the poll's own `pull_request_states` comment describes.
    /// That gap is worse than the defect it fixes: `sends_now` is live from
    /// the keypress, so until the widget catches up the chord the hint calls
    /// "newline" neither sends nor inserts one.
    ///
    /// Only entities that already exist need this. One built after the
    /// change reads the surface in [`Self::panel_prompt_input`] and is
    /// correct on arrival, whether or not a tick has run since.
    pub(in crate::workspace_window) fn reconcile_panel_send_chord(&mut self,
                                                                  cx: &mut Context<Self>)
                                                                  -> bool {
        let shift_to_send = crate::settings_global::read(cx).agent_panel_shift_enter_sends;
        if shift_to_send == self.panel_input_send_chord {
            return false;
        }
        self.panel_input_send_chord = shift_to_send;
        let inputs: Vec<_> = self.panel_prompt_inputs.values().cloned().collect();
        for input in inputs {
            cx.update_entity(&input, |state, cx| {
                  state.set_submit_on_enter(!shift_to_send, cx);
              });
        }
        true
    }

    /// Gets or creates the prompt input entity for `id`'s panel, wired so
    /// `Enter`/`Shift+Enter` submits per `agent_panel_shift_enter_sends`
    /// (the other chord always inserts a newline) - see
    /// `render_panel_input_area`'s Send button tooltip for the matching
    /// user-facing hint.
    ///
    /// A composer is built for the setting as it stands, read here rather
    /// than taken from `panel_input_send_chord`: that field says what the
    /// composers that already exist were last set to, and waiting for the
    /// poll to move it would hand a brand-new composer a chord already known
    /// to be out of date. One `Arc` clone, once per agent - the cache hit
    /// above is what a render repeats.
    pub(in crate::workspace_window) fn panel_prompt_input(&mut self, id: Uuid,
                                                          window: &mut Window,
                                                          cx: &mut Context<Self>)
                                                          -> Entity<PanelInputState> {
        if let Some(input) = self.panel_prompt_inputs.get(&id) {
            return input.clone();
        }
        let shift_to_send = crate::settings_global::read(cx).agent_panel_shift_enter_sends;
        let max_rows = panel_input_max_rows(self.panel_input_expanded.contains(&id));
        let input = new_panel_input(shift_to_send, max_rows, window, cx);
        let palette = Palette::of(cx);
        let subscription =
            cx.subscribe_in(&input,
                            window,
                            move |view: &mut Self, input, event, window, cx| {
                                match event {
                                    // Read, not captured: see [`sends_now`].
                                    InputEvent::PressEnter { shift, .. }
                                        if sends_now(*shift, cx) =>
                                    {
                                        view.send_panel_prompt(id, window, cx);
                                    }
                                    // Every way text arrives reports
                                    // here - typing, paste, undo, redo,
                                    // cut, a drag of text and the
                                    // lookup's own insertion - so one
                                    // arm restyles for all of them.
                                    InputEvent::Change => {
                                        let palette = Palette::of(cx);
                                        // The buffer decides: a chip
                                        // this edit removed detaches
                                        // its row, and the restyle
                                        // below then draws the table
                                        // that is left.
                                        if view.reconcile_panel_attachments(id, cx) {
                                            view.restyle_panel_attachments(id, palette, cx);
                                        }
                                        view.restyle_panel_composer(id, palette, cx);
                                        cx.notify();
                                    }
                                    // Focus leaving the input closes the
                                    // slash lookup, per its dismissal rules
                                    // - a popup left open behind another
                                    // pane is exactly what the shared
                                    // dismissal path exists to prevent.
                                    InputEvent::Blur => {
                                        let input = input.clone();
                                        view.dismiss_panel_lookup(id, &input, cx);
                                        cx.notify();
                                    }
                                    _ => {}
                                }
                            });
        self.panel_prompt_inputs.insert(id, input.clone());
        self.panel_prompt_input_subscriptions
            .insert(id, subscription);
        // After the entity is in the map, since the styling reads it back
        // out: a draft restored into a fresh composer is styled here, with
        // no edit to trigger it.
        self.ensure_panel_styling(id, palette, cx);
        input
    }

    /// The prompt textarea's placeholder - just the prompt, not the key
    /// chord (see `panel_prompt_send_hint` for that, rendered below the
    /// textarea instead of inside it).
    pub(in crate::workspace_window) fn panel_prompt_placeholder() -> &'static str {
        "Send a message…"
    }

    /// The send-chord hint shown below the prompt textarea, naming the
    /// active chord per `agent_panel_shift_enter_sends`.
    pub(in crate::workspace_window) fn panel_prompt_send_hint(shift_to_send: bool) -> &'static str {
        if shift_to_send {
            "Shift+Enter to send, Enter for a newline"
        }
        else {
            "Enter to send, Shift+Enter for a newline"
        }
    }
}
