//! Getting a prompt from the composer into the agent: the input entity and
//! its send chord, attached context, the queue when a turn is already in
//! flight, and stopping one that is.
//!
//! The queue's own rules live in [`super::super::prompt_queue`]; this is
//! the window side - which entity holds the text, when it may be sent, and
//! what happens to a delivery result coming back off the runtime.

use std::sync::Arc;

use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::ClipboardEntry;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::ImageFormat;
use gpui_kit::PathPromptOptions;
use gpui_kit::Window;
use gpui_kit::component::WindowExt;
use gpui_kit::component::input::Editor;
use gpui_kit::component::input::EditorState;
use gpui_kit::component::input::InputEvent;
use uuid::Uuid;

use crate::composer_style::Palette;
use crate::panel_session;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::prompt_queue;
use crate::workspace_window::prompt_queue::PromptOrigin;
use crate::workspace_window::prompt_queue::QueuedPanelPrompt;

/// How far the prompt box grows with its content before it starts
/// scrolling, collapsed and expanded. It auto-grows rather than sitting at
/// a fixed height: a fixed height fights the textarea's own layout, so a
/// second line made it scroll and jump on every keystroke instead of
/// simply getting taller.
pub(crate) const PANEL_INPUT_ROWS_COLLAPSED: usize = 6;
pub(crate) const PANEL_INPUT_ROWS_EXPANDED: usize = 20;

/// The state type behind the panel composer.
///
/// Named once so the widget the composer is built on is a single
/// declaration rather than a type repeated across the window.
///
/// It is an `EditorState` and not a `TextareaState` for one reason:
/// styled ranges. Decorations are stored in `state.extras`, keyed off the
/// **mode marker**, and `TextareaMode`'s extras are `()`, whose default
/// `decoration_layers()` is empty - so a textarea cannot carry a
/// decoration, ever. What the composer is *not* is a code editor; see
/// [`new_panel_input`] for how that is arranged.
pub(crate) type PanelInputState = EditorState;

/// The widget that draws [`PanelInputState`].
pub(crate) type PanelInput = Editor;

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
    /// Opens the native file/image picker and attaches the chosen paths to
    /// `id`'s pending message, per `knot-ui-conventions`' native-picker
    /// rule (`cx.prompt_for_paths` over an in-app file browser).
    pub(in crate::workspace_window) fn add_panel_context(&mut self, id: Uuid,
                                                         cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions { files:       true,
                                                             directories: false,
                                                             multiple:    true,
                                                             prompt:      Some("Attach".into()), });
        let this = cx.entity();
        cx.spawn(async move |_this, cx| {
              let Ok(Ok(Some(paths))) = receiver.await
              else {
                  return;
              };
              cx.update(|app| {
                    this.update(app, |view, cx| {
                            for path in paths {
                                view.panel_pending_context
                                    .entry(id)
                                    .or_default()
                                    .push(path.clone());
                                view.queue_attachment_reference(id, path);
                            }
                            view.restyle_panel_attachments(id, Palette::of(cx), cx);
                            cx.notify();
                        });
                });
          })
          .detach();
    }

    /// Reads an image off the system clipboard (e.g. a pasted screenshot)
    /// and attaches it to `id`'s pending message the same way
    /// `add_panel_context` attaches a picked file, since ACP's
    /// `session/prompt` here only carries text plus attached paths - see
    /// `render_panel_input_area`'s `capture_action::<Paste>` wiring.
    /// Returns `false` (leaving the paste to the textarea's own text-paste
    /// handling) when the clipboard holds no image.
    pub(in crate::workspace_window) fn paste_clipboard_image_context(&mut self, id: Uuid,
                                                                     cx: &mut App)
                                                                     -> bool {
        let Some(item) = cx.read_from_clipboard()
        else {
            return false;
        };
        let mut attached = false;
        for entry in item.entries {
            let ClipboardEntry::Image(image) = entry
            else {
                continue;
            };
            let extension = match image.format {
                ImageFormat::Png => "png",
                ImageFormat::Jpeg => "jpg",
                ImageFormat::Webp => "webp",
                ImageFormat::Gif => "gif",
                ImageFormat::Svg => "svg",
                ImageFormat::Bmp => "bmp",
                _ => "png",
            };
            let path = std::env::temp_dir().join(format!("knot-paste-{}.{extension}", image.id));
            if std::fs::write(&path, &image.bytes).is_ok() {
                self.panel_pending_context
                    .entry(id)
                    .or_default()
                    .push(path.clone());
                self.queue_attachment_reference(id, path);
                attached = true;
            }
        }
        if attached {
            self.restyle_panel_attachments(id, Palette::of(cx), cx);
        }
        attached
    }

    /// Removes one attached path from `id`'s pending context by index.
    /// Deletes the chip rather than the row: the deletion reports as an
    /// ordinary edit, and the reconciliation that follows every edit is
    /// what drops the row. One direction, so the strip and the buffer
    /// cannot disagree about which of them was right.
    ///
    /// The row is removed directly only when there is no chip to delete -
    /// a buffer that never received one, which is what an attachment
    /// dismissed before its deferred insertion ran looks like.
    pub(in crate::workspace_window) fn remove_panel_context(&mut self, id: Uuid, index: usize,
                                                            window: &mut Window, cx: &mut App) {
        let Some(path) = self.panel_pending_context
                             .get(&id)
                             .and_then(|paths| paths.get(index))
                             .cloned()
        else {
            return;
        };
        if self.remove_attachment_reference(id, &path, window, cx) {
            return;
        }
        if let Some(paths) = self.panel_pending_context.get_mut(&id)
           && index < paths.len()
        {
            paths.remove(index);
        }
        if let Some(queued) = self.panel_pending_attachments.get_mut(&id) {
            queued.retain(|waiting| waiting != &path);
        }
        self.restyle_panel_attachments(id, Palette::of(cx), cx);
    }

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

    /// Resolves the selected agent's pending permission request (if any,
    /// and if it's a Panel-mode agent) and answers it with `decision` -
    /// backs the allow/deny keybindings.
    pub(in crate::workspace_window) fn answer_selected_permission(&self,
                                                                  decision: knot_acp::PermissionDecision)
    {
        let Some(id) = self.selected_agent
        else {
            return;
        };
        if self.store.lock().agent(id).map(|agent| agent.view_mode)
           != Some(knot_core::ViewMode::Panel)
        {
            return;
        }
        let Some(slot) = self.panel_sessions.get(&id)
        else {
            return;
        };
        let slot = slot.lock();
        let panel_session::PanelSessionSlot::Ready(handle) = &*slot
        else {
            return;
        };
        let request = handle.state().lock().pending_permission.clone();
        if let Some(request) = request {
            handle.answer_permission(&request, decision);
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
    /// every frame forever.
    ///
    /// Called from `prepare_frame`, beside the window's other per-frame
    /// memos and for the same reason as `refresh_terminal_font`: it runs
    /// before the tree is built rather than during it, and it costs a
    /// `bool` compare on every frame that changed nothing.
    pub(in crate::workspace_window) fn reconcile_panel_send_chord(&mut self,
                                                                  cx: &mut Context<Self>) {
        let shift_to_send = crate::settings_global::read(cx).agent_panel_shift_enter_sends;
        if shift_to_send == self.panel_input_send_chord {
            return;
        }
        self.panel_input_send_chord = shift_to_send;
        let inputs: Vec<_> = self.panel_prompt_inputs.values().cloned().collect();
        for input in inputs {
            cx.update_entity(&input, |state, cx| {
                  state.set_submit_on_enter(!shift_to_send, cx);
              });
        }
    }

    /// Gets or creates the prompt input entity for `id`'s panel, wired so
    /// `Enter`/`Shift+Enter` submits per `agent_panel_shift_enter_sends`
    /// (the other chord always inserts a newline) - see
    /// `render_panel_input_area`'s Send button tooltip for the matching
    /// user-facing hint.
    ///
    /// A composer built here is built for the chord the frame already
    /// settled on, and `panel_input_send_chord` is that value rather than a
    /// fresh read: `reconcile_panel_send_chord` ran in `prepare_frame`, so
    /// re-reading the surface here would only be a second chance to
    /// disagree with the composers already on screen.
    pub(in crate::workspace_window) fn panel_prompt_input(&mut self, id: Uuid,
                                                          window: &mut Window,
                                                          cx: &mut Context<Self>)
                                                          -> Entity<PanelInputState> {
        if let Some(input) = self.panel_prompt_inputs.get(&id) {
            return input.clone();
        }
        let shift_to_send = self.panel_input_send_chord;
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

    /// Returns a queued message to the composer so the user can change it.
    ///
    /// The entry leaves the queue and does not hold its place: edited text
    /// is sent as a new prompt, behind whatever is still waiting. Text the
    /// user already typed is the only thing this can destroy, so replacing
    /// it asks first.
    pub(in crate::workspace_window) fn edit_queued_prompt(&mut self, id: Uuid, prompt_id: Uuid,
                                                          window: &mut Window,
                                                          cx: &mut Context<Self>) {
        let Some(input) = self.panel_prompt_inputs.get(&id).cloned()
        else {
            return;
        };
        if !prompt_queue::needs_replace_confirmation(&input.read(cx).value()) {
            self.take_queued_prompt_into(&input, id, prompt_id, window, cx);
            return;
        }
        let view = cx.entity();
        window.open_alert_dialog(cx, move |alert, _, _| {
                  let view = view.clone();
                  let input = input.clone();
                  alert.title(knot_core::l10n::t("panel.replace_composer_title"))
                       .description(knot_core::l10n::t("panel.replace_composer_body"))
                       .confirm()
                       .on_ok(move |_, window, app| {
                           view.update(app, |view, cx| {
                                   view.take_queued_prompt_into(&input, id, prompt_id, window, cx);
                               });
                           true
                       })
              });
    }

    /// Moves `prompt_id` out of `id`'s queue and into `input`, focused and
    /// ready to edit.
    ///
    /// A message already in flight is left where it is, so a delivery that
    /// starts between the click and the confirmation cannot be pulled back
    /// out from under the agent.
    pub(in crate::workspace_window) fn take_queued_prompt_into(&mut self,
                                                               input: &Entity<PanelInputState>,
                                                               id: Uuid, prompt_id: Uuid,
                                                               window: &mut Window,
                                                               cx: &mut Context<Self>) {
        let Some(queue) = self.panel_prompt_queues.get_mut(&id)
        else {
            return;
        };
        let Some(text) = prompt_queue::take(queue, prompt_id)
        else {
            return;
        };
        cx.update_entity(input, |state, cx| {
              state.set_value(text, window, cx);
              state.focus(window, cx);
          });
        cx.notify();
    }

    /// Reads and clears `id`'s prompt input, then hands it to
    /// [`Self::deliver_panel_prompt`].
    pub(in crate::workspace_window) fn send_panel_prompt(&mut self, id: Uuid,
                                                         window: &mut Window,
                                                         cx: &mut Context<Self>) {
        let Some(input) = self.panel_prompt_inputs.get(&id).cloned()
        else {
            return;
        };
        let raw = input.read(cx).value().to_string();
        // Before the trim, and before anything reaches the session: the
        // trigger is byte 0 of the *raw* buffer, which is what makes a single
        // leading space the escape for a prompt that has to begin with a
        // literal `!`. Trimming first would make ` !hello` executable and
        // leave the user no way to say it.
        if let Some(command) = crate::panel_commands::shell_command(&raw) {
            if !self.run_panel_shell_command(id, command.to_owned(), cx) {
                return;
            }
            cx.update_entity(&input, |state, cx| {
                  state.set_value("", window, cx);
              });
            cx.notify();
            return;
        }
        // `!` with nothing after it is neither a command nor a message. It
        // has to be refused here too and not only by the send control: the
        // Enter chord reaches this directly, and a lone `!` would otherwise
        // trim to a non-empty string and go to the agent as a prompt.
        if crate::panel_commands::has_shell_trigger(&raw) {
            return;
        }

        let mut text = raw.trim().to_string();
        if text.is_empty() {
            return;
        }
        // Attached context has no dedicated ACP content-block support here
        // (`knot-acp`'s `session/prompt` only sends a single text block),
        // so each path rides along as its own line rather than inventing
        // an unverified resource-attachment wire shape.
        let context = self.panel_pending_context.remove(&id).unwrap_or_default();
        for path in &context {
            text.push_str("\n\nAttached: ");
            text.push_str(&path.to_string_lossy());
        }
        // Every `!` command run since the last prompt rides along here, in
        // the order they were submitted - the only path by which a shell
        // result reaches the agent.
        text.push_str(&self.take_panel_shell_context(id));
        if !self.deliver_panel_prompt(id, text, PromptOrigin::User) {
            return;
        }
        cx.update_entity(&input, |state, cx| {
              state.set_value("", window, cx);
          });
        cx.notify();
    }

    /// Sends `text` through `id`'s live ACP session, or queues it when that
    /// session is mid-turn or blocked on a pending permission, per
    /// `acp-panel-ui`'s "blocking further prompt submission until answered"
    /// requirement.
    ///
    /// Split out of [`Self::send_panel_prompt`] so a broadcast reaches an
    /// agent by exactly the path the agent's own composer uses: the
    /// queueing, the recording of the user's message in the conversation
    /// and the error reporting are one implementation rather than two that
    /// can drift.
    ///
    /// `origin` says what produced the prompt. It is recorded on the queued
    /// entry so a prompt Knot sent of its own accord stays distinguishable
    /// from one the user typed, per `mcp-messaging`'s "Inbox nudges
    /// preserve interrupted session work" - the alternative, matching the
    /// nudge's wording, misclassifies a user who pastes it.
    ///
    /// Returns whether `id` has a panel session at all. An agent with none
    /// is skipped, and its composer is left untouched.
    pub(in crate::workspace_window) fn deliver_panel_prompt(&mut self, id: Uuid, text: String,
                                                            origin: PromptOrigin)
                                                            -> bool {
        let Some(slot) = self.panel_sessions.get(&id)
        else {
            return false;
        };
        let session = {
            let guard = slot.lock();
            match &*guard {
                panel_session::PanelSessionSlot::Ready(handle) => {
                    let state = handle.state();
                    let state = state.lock();
                    let ready = state.pending_permission.is_none() && !state.turn_active;
                    drop(state);
                    ready.then(|| {
                             handle.record_user_message(text.clone());
                             (handle.session(), handle.recorder())
                         })
                }
                _ => None,
            }
        };
        let Some((session, recorder)) = session
        else {
            self.panel_prompt_queues
                .entry(id)
                .or_default()
                .push(QueuedPanelPrompt::new(text, origin));
            return true;
        };
        let _runtime_guard = self.runtime.enter();
        self.runtime.spawn(async move {
                        if let Err(error) = session.prompt(&text).await {
                            // Shown under the prompt it belongs to, and
                            // it ends the turn - an error response is all
                            // the answer this prompt gets, so the
                            // composer must not stay blocked waiting for
                            // a `TurnEnd` that will never arrive.
                            recorder.error(knot_core::l10n::t_with("panel.error_answer",
                                                                   &[("error",
                                                                      &error.to_string())]));
                            eprintln!("failed to send panel prompt: {error}");
                        }
                    });
        true
    }

    pub(in crate::workspace_window) fn stop_panel_prompt(&mut self, id: Uuid,
                                                         cx: &mut Context<Self>) {
        if !self.panel_stopping.insert(id) {
            return;
        }
        let Some(slot) = self.panel_sessions.get(&id).cloned()
        else {
            self.panel_stopping.remove(&id);
            return;
        };
        let session = {
            let guard = slot.lock();
            match &*guard {
                panel_session::PanelSessionSlot::Ready(handle) => Some(handle.session()),
                _ => None,
            }
        };
        let Some(session) = session
        else {
            self.panel_stopping.remove(&id);
            return;
        };
        let _runtime_guard = self.runtime.enter();
        self.runtime.spawn(async move {
                        if let Err(error) = session.cancel().await {
                            eprintln!("failed to cancel agent {id}'s turn: {error}");
                        }
                    });
        cx.notify();
    }

    /// Returns whether a queued prompt was just picked up (moved to
    /// `in_flight`) - the caller feeds this into `deliver_waiting_prompts`'s
    /// dirty check, since flipping that flag changes what the queue row
    /// shows (waiting vs. in flight) with nothing else marking the frame
    /// dirty.
    pub(in crate::workspace_window) fn drain_panel_prompt(&mut self, id: Uuid) -> bool {
        let Some(slot) = self.panel_sessions.get(&id)
        else {
            return false;
        };
        let mut candidate = || -> Option<_> {
            let guard = slot.lock();
            let panel_session::PanelSessionSlot::Ready(handle) = &*guard
            else {
                return None;
            };
            let state = handle.state();
            let state = state.lock();
            if state.pending_permission.is_some() || state.turn_active {
                return None;
            }
            drop(state);
            let queue = self.panel_prompt_queues.get_mut(&id)?;
            let prompt = queue.first_mut()?;
            if prompt.failed || prompt.in_flight {
                return None;
            }
            prompt.in_flight = true;
            handle.record_user_message(prompt.text.clone());
            Some((handle.session(), handle.recorder(), prompt.text.clone(), prompt.id))
        };
        let Some((session, recorder, text, prompt_id)) = candidate()
        else {
            return false;
        };
        let results = Arc::clone(&self.panel_prompt_results);
        self.runtime.spawn(async move {
                        let result = session.prompt(&text)
                                            .await
                                            .map_err(|error| error.to_string());
                        if let Err(error) = &result {
                            recorder.error(knot_core::l10n::t_with("panel.error_answer",
                                                                   &[("error",
                                                                      &error.to_string())]));
                        }
                        {
                            let mut results = results.lock();
                            results.push((id, prompt_id, result));
                        }
                    });
        true
    }
}
