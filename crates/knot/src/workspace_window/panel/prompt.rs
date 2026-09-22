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
use gpui_kit::component::input::InputEvent;
use gpui_kit::component::input::TextareaState;
use uuid::Uuid;

use crate::panel_session;
use crate::workspace_window::PANEL_INPUT_ROWS_COLLAPSED;
use crate::workspace_window::PANEL_INPUT_ROWS_EXPANDED;
use crate::workspace_window::PromptOrigin;
use crate::workspace_window::QueuedPanelPrompt;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::prompt_queue;

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
                            view.panel_pending_context
                                .entry(id)
                                .or_default()
                                .extend(paths);
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
                self.panel_pending_context.entry(id).or_default().push(path);
                attached = true;
            }
        }
        attached
    }

    /// Removes one attached path from `id`'s pending context by index.
    pub(in crate::workspace_window) fn remove_panel_context(&mut self, id: Uuid, index: usize) {
        if let Some(paths) = self.panel_pending_context.get_mut(&id)
           && index < paths.len()
        {
            paths.remove(index);
        }
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
        let max_rows = if expanded {
            PANEL_INPUT_ROWS_EXPANDED
        }
        else {
            PANEL_INPUT_ROWS_COLLAPSED
        };
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

    /// Gets or creates the prompt input entity for `id`'s panel, wired so
    /// `Enter`/`Shift+Enter` submits per `agent_panel_shift_enter_sends`
    /// (the other chord always inserts a newline) - see
    /// `render_panel_input_area`'s Send button tooltip for the matching
    /// user-facing hint.
    pub(in crate::workspace_window) fn panel_prompt_input(&mut self, id: Uuid,
                                                          window: &mut Window,
                                                          cx: &mut Context<Self>)
                                                          -> Entity<TextareaState> {
        if let Some(input) = self.panel_prompt_inputs.get(&id) {
            return input.clone();
        }
        let shift_to_send = self.settings.agent_panel_shift_enter_sends;
        let placeholder = Self::panel_prompt_placeholder();
        let max_rows = if self.panel_input_expanded.contains(&id) {
            PANEL_INPUT_ROWS_EXPANDED
        }
        else {
            PANEL_INPUT_ROWS_COLLAPSED
        };
        let input = cx.new(|cx| {
                          TextareaState::new(window, cx).placeholder(placeholder)
                                                        .submit_on_enter(!shift_to_send)
                                                        .auto_grow(1, max_rows)
                      });
        let subscription = cx.subscribe_in(&input,
                                           window,
                                           move |view: &mut Self, _, event, window, cx| {
                                               if let InputEvent::PressEnter { shift, .. } = event
                                                  && *shift == shift_to_send
                                               {
                                                   view.send_panel_prompt(id, window, cx);
                                               }
                                           });
        self.panel_prompt_inputs.insert(id, input.clone());
        self.panel_prompt_input_subscriptions
            .insert(id, subscription);
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
                                                               input: &Entity<TextareaState>,
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
        let mut text = input.read(cx).value().trim().to_string();
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
                            recorder.error(format!("The agent could not answer: {error}"));
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
                            recorder.error(format!("The agent could not answer: {error}"));
                        }
                        {
                            let mut results = results.lock();
                            results.push((id, prompt_id, result));
                        }
                    });
        true
    }
}
