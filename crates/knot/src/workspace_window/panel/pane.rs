//! The content pane for a selected agent: the panel conversation, the
//! markdown viewer that can take it over, and the placeholder for an agent
//! that is not running.
//!
//! The conversation itself is drawn by [`crate::panel_view`] from
//! [`crate::panel_state`]; this owns the pane around it and the
//! virtualized list's reconciliation.

use std::path::Path;
use std::sync::Arc;

use gpui_kit::ClickEvent;
use gpui_kit::Context;
use gpui_kit::FollowMode;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ListAlignment;
use gpui_kit::ListState;
use gpui_kit::ParentElement;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::assets::IconName;
use gpui_kit::base::StyledExt;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::div;
use gpui_kit::px;
use gpui_kit::rgb;
use parking_lot::Mutex;
use uuid::Uuid;

use crate::app_support::single_line;
use crate::panel_session;
use crate::panel_view;
use crate::workspace_window::WorkspaceWindow;

impl WorkspaceWindow {
    /// Renders the markdown pane for `id`, which takes over the content
    /// area while the agent has a markdown file open.
    ///
    /// The `display-markdown` MCP tool and the "Markdown Files" context
    /// menu item both set that file; until this existed, both wrote state
    /// no UI ever read, so an agent calling the tool appeared to be
    /// ignored. Closing the pane clears the file but keeps the history, so
    /// the menu can bring it back.
    /// The content pane for an agent's diagram: a committed task plan, or
    /// anything else an agent showed with `view-mermaid`.
    ///
    /// Same chrome as the markdown pane, because it is the same kind of
    /// thing - something an agent put in front of the user, which the user
    /// closes when done with it.
    pub(in crate::workspace_window) fn render_mermaid_pane(&self, id: Uuid, source: &str,
                                                           title: Option<&str>,
                                                           cx: &mut Context<Self>)
                                                           -> gpui_kit::AnyElement {
        let heading = title.map(str::to_string)
                           .unwrap_or_else(|| knot_core::l10n::t("plan.title"));
        // Nothing parseable is not an error: the agent showed something
        // this build cannot draw, and saying so beats an empty pane, which
        // reads as a bug.
        let body = crate::plan_view::plan_diagram(source, cx).unwrap_or_else(|| {
                                                                 div().text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(knot_core::l10n::t("plan.empty"))
                            .into_any_element()
                                                             });
        v_flex()
            .size_full()
            .child(
                h_flex()
                    .w_full()
                    .flex_shrink_0()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .font_semibold()
                            .child(single_line(&heading)),
                    )
                    .child(
                        Button::new("mermaid-pane-close")
                            .icon(IconName::Close)
                            .ghost()
                            .small()
                            .tooltip("Close")
                            .on_click(cx.listener(move |view, _, _window, cx| {
                                {
                                    let mut store = view.store.lock();
                                    if let Err(error) = store.clear_mermaid_panel(id) {
                                        eprintln!("failed to close the diagram panel: \
                                                   {error}");
                                    }
                                }
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .id(("mermaid-pane", id.as_u128() as u64))
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .min_w_0()
                    .overflow_scroll()
                    .p_4()
                    .child(body),
            )
            .into_any_element()
    }

    pub(in crate::workspace_window) fn render_markdown_pane(&self, id: Uuid, file: &Path,
                                                            cx: &mut Context<Self>)
                                                            -> gpui_kit::AnyElement {
        let title = file.file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| file.to_string_lossy().into_owned());
        // Read at render time rather than cached: the file is written by
        // an agent that may still be editing it, and re-reading is what
        // makes a second `display-markdown` of the same path show the new
        // content.
        let body = std::fs::read_to_string(file).unwrap_or_else(|error| {
                       format!("Could not read `{}`:\n\n```\n{error}\n```", file.display())
                   });
        v_flex()
            .size_full()
            .child(
                h_flex()
                    .w_full()
                    .flex_shrink_0()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .px_3()
                    .py_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .font_semibold()
                            .child(single_line(&title)),
                    )
                    .child(
                        Button::new("markdown-pane-close")
                            .icon(IconName::Close)
                            .ghost()
                            .small()
                            .tooltip(knot_core::l10n::t("panel.close"))
                            .on_click(cx.listener(move |view, _, _window, cx| {
                                {
                                    let mut store = view.store.lock();
                                    if let Err(error) = store.clear_markdown_panel(id) {
                                        eprintln!("failed to close the markdown panel: \
                                                   {error}");
                                    }
                                }
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .id(("markdown-pane", id.as_u128() as u64))
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .min_w_0()
                    .overflow_y_scroll()
                    .p_4()
                    // The same constructor the panel's assistant messages
                    // use, with the same families and body size: the spec
                    // requires the two Markdown surfaces to render alike.
                    .child(crate::markdown_view::markdown_view(
                        ("markdown-pane-body", id.as_u128() as u64),
                        body,
                        cx.theme().font_family.clone(),
                        self.settings.title_font_name.clone().into(),
                        px(self.settings.markdown_font_size as f32),
                    )),
            )
            .into_any_element()
    }

    /// The content pane for a selected agent that is not running: a
    /// `passive` agent nobody has started, or one that was deactivated.
    ///
    /// It exists because the alternative reads as a bug: an empty pane on an
    /// agent whose state dot says Idle is exactly what a hung agent looks
    /// like. Naming why it is not running, and that selecting it starts it,
    /// is the same guard the editor's activation hint gives from the other
    /// side.
    pub(in crate::workspace_window) fn render_stopped_pane(&self, name: String,
                                                           cx: &Context<Self>)
                                                           -> gpui_kit::AnyElement {
        v_flex().size_full()
                .items_center()
                .justify_center()
                .gap_1()
                .child(div().text_color(cx.theme().foreground)
                            .child(format!("{name} is not running")))
                .child(div().text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(knot_core::l10n::t("panel.select_agent_to_start")))
                .into_any_element()
    }

    /// Renders the Panel-mode content pane for `id`: a connecting/failed
    /// placeholder, or the folded conversation plus a prompt input once
    /// the ACP session is ready. Starts the session if it isn't already
    /// running.
    pub(in crate::workspace_window) fn render_panel_pane(&mut self, id: Uuid,
                                                         window: &mut Window,
                                                         cx: &mut Context<Self>)
                                                         -> gpui_kit::AnyElement {
        self.ensure_panel_session(id);
        // The first place after an attachment arrives with a window to
        // edit the buffer with. Nothing else in the frame depends on the
        // insertion, so doing it here rather than threading a window back
        // through three arrival paths costs one frame and no correctness.
        if self.insert_queued_attachments(id, window, cx) {
            self.restyle_panel_attachments(id, crate::composer_style::Palette::of(cx), cx);
        }
        let Some(slot) = self.panel_sessions.get(&id)
        else {
            return div().into_any_element();
        };
        let slot_guard = slot.lock();
        match &*slot_guard {
            panel_session::PanelSessionSlot::Connecting(progress) => {
                let step = progress.lock().label();
                v_flex().size_full()
                        .items_center()
                        .justify_center()
                        .gap_1()
                        .child(div().text_color(rgb(0x9CA3AF))
                                    .child(knot_core::l10n::t("panel.connecting")))
                        .child(div().text_xs().text_color(rgb(0x6B7280)).child(step))
                        .into_any_element()
            }
            panel_session::PanelSessionSlot::Failed(message) => {
                let message = message.clone();
                drop(slot_guard);
                // A failed connect is often transient - a loaded machine,
                // an adapter slow to answer `initialize` - so offer the
                // retry rather than making the user remove and re-add the
                // agent to get another attempt.
                v_flex()
                    .size_full()
                    .p_4()
                    .gap_3()
                    .items_start()
                    .child(
                        div()
                            .text_color(rgb(0xEF4444))
                            .child(format!("Failed to connect: {message}")),
                    )
                    .child(
                        // An icon with a tooltip, like every other panel
                        // control - the failure text above it already says
                        // what went wrong, so the button does not have to
                        // repeat the offer in words.
                        Button::new("panel-retry-connect")
                            .icon(gpui_kit::component::Icon::new(
                                gpui_kit::assets::IconName::RefreshCw,
                            ))
                            .tooltip(knot_core::l10n::t("panel.retry_connect"))
                            .accessibility_label(knot_core::l10n::t("panel.retry_connect"))
                            .primary()
                            .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                                view.retry_panel_session(id);
                                cx.notify();
                            })),
                    )
                    .into_any_element()
            }
            panel_session::PanelSessionSlot::Ready(handle) => {
                let state_arc = handle.state();
                let state = state_arc.lock();
                let blocked = state.pending_permission.is_some();
                let session_arc = Arc::clone(slot);
                let pending = state.pending_permission.clone();
                let on_decision = move |decision: knot_acp::PermissionDecision| {
                    let Some(request) = &pending
                    else {
                        return;
                    };
                    if let panel_session::PanelSessionSlot::Ready(handle) = &*session_arc.lock() {
                        handle.answer_permission(request, decision);
                    }
                };
                let track_slot = Arc::clone(slot);
                let on_toggle_track = move || {
                    if let panel_session::PanelSessionSlot::Ready(handle) = &*track_slot.lock() {
                        handle.toggle_tracking();
                    }
                };
                let tool_call_slot = Arc::clone(slot);
                let on_toggle_tool_call = move |tool_call_id: String| {
                    if let panel_session::PanelSessionSlot::Ready(handle) = &*tool_call_slot.lock()
                    {
                        handle.toggle_tool_call(&tool_call_id);
                    }
                };
                let tool_run_slot = Arc::clone(slot);
                let on_toggle_tool_run = move |head_id: String| {
                    if let panel_session::PanelSessionSlot::Ready(handle) = &*tool_run_slot.lock() {
                        handle.toggle_tool_run(head_id);
                    }
                };
                let manual_slot = Arc::clone(slot);
                let on_manual_scroll = move || {
                    if let panel_session::PanelSessionSlot::Ready(handle) = &*manual_slot.lock() {
                        handle.clear_tracking();
                    }
                };
                let follow_slot = Arc::clone(slot);
                let list_slot = Arc::clone(slot);
                let should_follow = state.turn_active && state.tracking;
                let turn_active = state.turn_active;
                let config_options = state.config_options.clone();
                let permission_risk = Self::find_config_option(
                    &config_options,
                    &["mode", "permission_mode", "permission-mode"],
                )
                .and_then(|option| option.current_value.as_str())
                .map(|value| panel_view::permission_risk_level(value, "Permission"))
                .unwrap_or(panel_view::RiskLevel::Neutral);
                let theme = cx.theme();
                let panel_style =
                    panel_view::PanelStyle { permission_risk,
                                             markdown_font_size: px(self.settings.markdown_font_size
                                                                    as f32),
                                             mono_font_family: theme.mono_font_family.clone(),
                                             ui_font_family: theme.font_family.clone(),
                                             title_font_family: self.settings
                                                                    .title_font_name
                                                                    .clone()
                                                                    .into(),
                                             danger_color: theme.danger,
                                             info_color: theme.info,
                                             border_color: theme.border,
                                             card_color: theme.secondary,
                                             prompt_color: theme.primary,
                                             prompt_foreground: theme.primary_foreground,
                                             compact_tool_calls:
                                                 self.settings.agent_panel_compact_tool_calls };
                drop(state);
                drop(slot_guard);
                // Reconcile the virtualized list with the folded state:
                // splice only the rows that changed, then mirror the track
                // toggle's follow state onto the list. `Tail` while the
                // in-flight response is tracked; `Normal` otherwise, so a
                // toggled-off response stays put *even at the tail* - the
                // whole reason for hand-rolling on `ListState` rather than
                // using a scroller whose follow mode re-engages itself.
                let list = self.panel_list(id, list_slot);
                let known = self.panel_list_row_counts.get(&id).copied().unwrap_or(0);
                let row_count = {
                    let state = state_arc.lock();
                    let count = panel_view::sync_row_count(&list, known, &state);
                    self.panel_list_row_counts.insert(id, count);
                    count
                };
                if should_follow {
                    if !list.is_following_tail() {
                        list.set_follow_mode(FollowMode::Tail);
                    }
                }
                else {
                    list.set_follow_mode(FollowMode::Normal);
                }
                let pending_context = self.panel_pending_context
                                          .get(&id)
                                          .cloned()
                                          .unwrap_or_default();
                let queued_prompts = self.panel_prompt_queues
                                         .get(&id)
                                         .cloned()
                                         .unwrap_or_default();
                let expanded = self.panel_input_expanded.contains(&id);
                let input = self.panel_prompt_input(id, window, cx);
                // Asked of the tail row's position rather than of
                // `is_scrolled_to_end`, which needs a total content height
                // the virtualized list does not have: it measures only the
                // rows near the viewport, so it answered `None` for every
                // conversation longer than a screen and the control went
                // missing. See `panel_view::scroll`.
                let scrolled_up = panel_view::scrolled_away_from_tail(&list, row_count);
                let list_to_bottom = list.clone();
                v_flex()
                    .size_full()
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .min_h_0()
                            .child(panel_view::render_panel(
                                Arc::clone(&state_arc),
                                list.clone(),
                                &panel_style,
                                panel_view::PanelCallbacks::new(
                                    on_decision,
                                    on_toggle_track,
                                    on_toggle_tool_call,
                                    on_toggle_tool_run,
                                    on_manual_scroll,
                                ),
                            ))
                            .children(scrolled_up.then(|| {
                                div().absolute().bottom_3().right_4().child(
                                    Button::new("panel-scroll-to-bottom")
                                        .icon(IconName::ChevronDown)
                                        .tooltip(knot_core::l10n::t("panel.scroll_to_latest"))
                                        .small()
                                        .on_click(move |_: &ClickEvent, _, _| {
                                            list_to_bottom.scroll_to_end();
                                            // Jumping to the end also
                                            // resumes following new
                                            // output, which is what the
                                            // control implies.
                                            if let panel_session::PanelSessionSlot::Ready(
                                                handle,
                                            ) = &*follow_slot.lock()
                                            {
                                                handle.set_tracking(true);
                                            }
                                        }),
                                )
                            })),
                    )
                    .child(self.render_panel_input_area(
                        id,
                        &input,
                        &pending_context,
                        &queued_prompts,
                        expanded,
                        blocked,
                        turn_active,
                        &config_options,
                        cx,
                    ))
                    .into_any_element()
            }
        }
    }

    /// Gets or creates the conversation's virtualized list for `id`'s panel.
    ///
    /// `slot` is the panel session slot the list's scroll handler clears
    /// tracking through: a user scroll (wheel or scrollbar drag) away from
    /// the tail fires the handler, which drops the in-flight response's
    /// auto-scroll - per the track toggle's "detect user-initiated scroll
    /// away from bottom" scenario - without a window/cx in the closure.
    /// The list is created empty; the caller reconciles its item count
    /// each frame (see `panel_view::sync_row_count`).
    pub(in crate::workspace_window) fn panel_list(&mut self, id: Uuid,
                                                  slot: Arc<Mutex<panel_session::PanelSessionSlot>>)
                                                  -> ListState {
        if let Some(list) = self.panel_lists.get(&id) {
            return list.clone();
        }
        let list = ListState::new(0, ListAlignment::Top, px(panel_view::LIST_OVERDRAW));
        list.set_scroll_handler(move |_event, _window, _cx| {
                if let panel_session::PanelSessionSlot::Ready(handle) = &*slot.lock() {
                    handle.clear_tracking();
                }
            });
        self.panel_lists.insert(id, list.clone());
        self.panel_list_row_counts.insert(id, 0);
        list
    }
}
