//! The panel prompt's slash lookup: the popup above the composer, the keys
//! that drive it, and the insertion that closes it.
//!
//! Contract: `openspec/specs/panel-slash-commands/spec.md` (proposed in
//! `openspec/changes/panel-slash-command-lookup`).
//!
//! The lookup completes text and nothing else. Inserting an entry rewrites
//! the slash token in the buffer; sending is the composer's existing path,
//! unchanged, so a completed prompt reaches the agent as ordinary text.
//!
//! Whether the popup is open is *derived*, not stored: the token under the
//! caret decides it on every render (see [`crate::panel_commands::token`]).
//! The only state kept per agent is the entry the user has moved to and the
//! token they last dismissed with Esc - see [`PanelLookup`].

use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::input::Enter;
use gpui_kit::component::input::Escape;
use gpui_kit::component::input::IndentInline;
use gpui_kit::component::input::MoveDown;
use gpui_kit::component::input::MoveUp;
use gpui_kit::div;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::px;
use uuid::Uuid;

use crate::app_support::single_line;
use crate::composer_scan::escape_token;
use crate::panel_commands::ActiveToken;
use crate::panel_commands::LookupMatch;
use crate::panel_commands::LookupRegistry;
use crate::panel_commands::Trigger;
use crate::panel_commands::active_token;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::element_key;
use crate::workspace_window::panel::mentions::MentionState;
use crate::workspace_window::panel::prompt::PanelInputState;

/// How many entries the popup shows at once before scrolling.
const LOOKUP_MAX_VISIBLE: usize = 8;

/// One agent's lookup state.
///
/// `registry` is the memoized scan: building it reads skill roots off disk,
/// which must not happen per keystroke (see the crate's "no I/O on the
/// render path" rule), so it is built on the first lookup for an agent and
/// dropped with the rest of that agent's state in `teardown_session`.
pub(in crate::workspace_window) struct PanelLookup {
    registry:  LookupRegistry,
    /// Index into the *filtered* list, clamped on use - the filter narrows
    /// under the selection as the user types.
    selected:  usize,
    /// The token text Esc closed the popup on. While the token still reads
    /// this way the popup stays shut; editing the token reopens it.
    dismissed: Option<String>,
}

impl PanelLookup {
    fn new(registry: LookupRegistry) -> Self {
        Self { registry,
               selected: 0,
               dismissed: None }
    }
}

impl WorkspaceWindow {
    /// The lookup's state for `id`, building its registry on first use.
    ///
    /// Reads skill roots off disk, so every caller must be a key handler or
    /// a first render for the agent - never a per-keystroke path.
    fn panel_lookup(&mut self, id: Uuid) -> &mut PanelLookup {
        if !self.panel_lookups.contains_key(&id) {
            let folder = {
                let store = self.store.lock();
                store.agent(id)
                     .map(|agent| std::path::PathBuf::from(&agent.folder))
            };
            let registry = LookupRegistry::for_agent(folder.as_deref());
            self.panel_lookups.insert(id, PanelLookup::new(registry));
        }
        self.panel_lookups
            .get_mut(&id)
            .expect("just inserted the lookup state")
    }

    /// The token under the caret in `input`, or `None` when there is none.
    fn panel_lookup_token(input: &Entity<PanelInputState>, cx: &gpui_kit::App)
                          -> Option<ActiveToken> {
        let state = input.read(cx);
        active_token(&state.value(), state.cursor())
    }

    /// The entries the open lookup would list for `id`, or `None` when the
    /// lookup is closed - no token, dismissed, or nothing matching.
    ///
    /// This is the single "is the popup open?" answer; the renderer and
    /// every key handler ask it rather than keeping a flag in step.
    fn panel_lookup_matches(&mut self, id: Uuid, input: &Entity<PanelInputState>,
                            cx: &gpui_kit::App)
                            -> Option<(ActiveToken, Vec<LookupMatch>)> {
        let token = Self::panel_lookup_token(input, cx)?;
        // The trigger under the caret decides which list this is, so the
        // two lookups cannot both be open: there is one caret.
        if token.trigger == Trigger::Mention {
            return self.panel_mention_matches(id, token);
        }
        let lookup = self.panel_lookup(id);
        // Typing past what Esc closed reopens the popup; retyping the same
        // token leaves it closed, so Esc is not undone by a redraw.
        if lookup.dismissed.as_deref() == Some(token.filter.as_str()) {
            return None;
        }
        lookup.dismissed = None;
        let matches: Vec<LookupMatch> = lookup.registry.matching(&token.filter);
        if matches.is_empty() {
            return None;
        }
        lookup.selected = lookup.selected.min(matches.len() - 1);
        Some((token, matches))
    }

    /// The `@` lookup's matches for `id`, or `None` when it is closed.
    ///
    /// Starting the listing is a side effect of asking: the first `@` an
    /// agent sees is what pays for the walk of its folder. While that
    /// runs the popup says so rather than showing an empty list, which
    /// would read as "no such file".
    fn panel_mention_matches(&mut self, id: Uuid, token: ActiveToken)
                             -> Option<(ActiveToken, Vec<LookupMatch>)> {
        let state = self.ensure_panel_mentions(id)?;
        if self.panel_lookup(id).dismissed.as_deref() == Some(token.filter.as_str()) {
            return None;
        }
        self.panel_lookup(id).dismissed = None;

        let matches = match state {
            MentionState::Gathering => {
                vec![LookupMatch::status(knot_core::l10n::t("panel.mentions_gathering"))]
            }
            MentionState::Ready { overflowed } => {
                let mentions = self.panel_mentions_for(id)?;
                let mut matches = mentions.registry().matching(&token.filter);
                matches.truncate(LOOKUP_MAX_VISIBLE);
                if matches.is_empty() {
                    return None;
                }
                if overflowed {
                    // Appended rather than replacing the list: matching
                    // still ran over what was gathered, and the row says
                    // the answer is incomplete rather than absent.
                    matches.push(LookupMatch::status(knot_core::l10n::t("panel.mentions_capped")));
                }
                matches
            }
        };
        let lookup = self.panel_lookup(id);
        lookup.selected = lookup.selected.min(matches.len() - 1);
        Some((token, matches))
    }

    /// The popup, when the lookup is open - a list above the prompt row.
    pub(in crate::workspace_window) fn render_panel_lookup(&mut self, id: Uuid,
                                                           input: &Entity<PanelInputState>,
                                                           cx: &mut Context<Self>)
                                                           -> Option<impl IntoElement + use<>> {
        let (token, matches) = self.panel_lookup_matches(id, input, cx)?;
        let trigger = token.trigger;
        let selected = self.panel_lookup(id).selected;
        let input = input.clone();

        let rows = matches.into_iter().enumerate().map(|(index, entry)| {
            let input = input.clone();
            let is_selected = index == selected;
            let is_status = entry.is_status;
            h_flex().id(("panel-lookup-entry", element_key(id).wrapping_add(index as u64)))
                    .w_full()
                    .min_w_0()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded(px(4.))
                    .when_selected(is_selected, cx)
                    .child(div().flex_shrink_0()
                                .font_family(cx.theme().mono_font_family.clone())
                                .child(if entry.is_status {
                                    entry.entry.token.clone()
                                }
                                else {
                                    format!("{}{}", trigger.char(), entry.entry.token)
                                }))
                    .child(div().flex_1()
                                .min_w_0()
                                .text_color(cx.theme().muted_foreground)
                                .child(single_line(&entry.entry.description)))
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.panel_lookup_select(id, index);
                        view.insert_panel_lookup_entry(id, &input, window, cx);
                    }))
                    .when(is_status, |row| row.cursor_default())
        });

        Some(v_flex().id(("panel-lookup", element_key(id)))
                     .w_full()
                     .min_w_0()
                     .max_h(px(LOOKUP_MAX_VISIBLE as f32 * 28.))
                     .overflow_y_scroll()
                     .gap_0p5()
                     .p_1()
                     .rounded(px(6.))
                     .border_1()
                     .border_color(cx.theme().border)
                     .bg(cx.theme().popover)
                     .children(rows)
                     // The keys are not discoverable from the list itself,
                     // and there is no empty state to put them in: a filter
                     // matching nothing dismisses the popup outright.
                     .child(div().px_2()
                                 .pt_1()
                                 .text_xs()
                                 .text_color(cx.theme().muted_foreground)
                                 .child(knot_core::l10n::t("panel.lookup_hint"))))
    }

    /// Moves the selection by `delta` entries, stopping at either end.
    fn panel_lookup_move(&mut self, id: Uuid, delta: isize, matches: usize) {
        let lookup = self.panel_lookup(id);
        let last = matches.saturating_sub(1);
        lookup.selected = match delta {
            delta if delta < 0 => lookup.selected.saturating_sub(delta.unsigned_abs()),
            delta => (lookup.selected + delta as usize).min(last),
        };
    }

    fn panel_lookup_select(&mut self, id: Uuid, index: usize) {
        self.panel_lookup(id).selected = index;
    }

    /// Closes the popup without touching the buffer, per Esc's scenario.
    pub(in crate::workspace_window) fn dismiss_panel_lookup(&mut self, id: Uuid,
                                                            input: &Entity<PanelInputState>,
                                                            cx: &gpui_kit::App) {
        let filter = Self::panel_lookup_token(input, cx).map(|token| token.filter);
        self.panel_lookup(id).dismissed = filter.or(Some(String::new()));
    }

    /// Replaces the slash token under the caret with the selected entry's
    /// token, leaving the rest of the buffer alone.
    fn insert_panel_lookup_entry(&mut self, id: Uuid, input: &Entity<PanelInputState>,
                                 window: &mut Window, cx: &mut Context<Self>) {
        let Some((token, matches)) = self.panel_lookup_matches(id, input, cx)
        else {
            return;
        };
        let selected = self.panel_lookup(id).selected.min(matches.len() - 1);
        // A status row says what the lookup is doing; it is not a thing
        // that can be inserted, and Enter on one must leave the buffer
        // alone rather than write the message into the prompt.
        if matches[selected].is_status {
            return;
        }
        replace_lookup_token(input,
                             token.range.clone(),
                             token.trigger,
                             &matches[selected].entry.token,
                             window,
                             cx);
        // The token now reads as the inserted entry; marking that dismissed
        // keeps the popup shut over the completed token instead of
        // reopening on the exact match the user just chose.
        self.panel_lookup(id).dismissed = Some(matches[selected].entry.token.clone());
        self.panel_lookup(id).selected = 0;
        cx.notify();
    }

    /// Wires the lookup's keys onto `element`, which must be an ancestor of
    /// the prompt textarea.
    ///
    /// These are capture-phase handlers: the textarea binds every one of
    /// these keys itself (Up/Down move the caret, Tab indents, Enter sends),
    /// so the lookup has to see them first and stop them going further -
    /// but only while it is open, or ordinary typing would lose those keys.
    pub(in crate::workspace_window) fn wire_panel_lookup_keys<E>(&self, element: E, id: Uuid,
                                                                 input: &Entity<PanelInputState>,
                                                                 cx: &mut Context<Self>)
                                                                 -> E
        where E: InteractiveElement {
        let entity = cx.entity();
        element.capture_action::<MoveUp>({
                   let (entity, input) = (entity.clone(), input.clone());
                   move |_, _, app| {
                       entity.update(app, |view, cx| {
                                 if let Some((_, matches)) =
                                     view.panel_lookup_matches(id, &input, cx)
                                 {
                                     view.panel_lookup_move(id, -1, matches.len());
                                     cx.stop_propagation();
                                     cx.notify();
                                 }
                             });
                   }
               })
               .capture_action::<MoveDown>({
                   let (entity, input) = (entity.clone(), input.clone());
                   move |_, _, app| {
                       entity.update(app, |view, cx| {
                                 if let Some((_, matches)) =
                                     view.panel_lookup_matches(id, &input, cx)
                                 {
                                     view.panel_lookup_move(id, 1, matches.len());
                                     cx.stop_propagation();
                                     cx.notify();
                                 }
                             });
                   }
               })
               .capture_action::<Escape>({
                   let (entity, input) = (entity.clone(), input.clone());
                   move |_, _, app| {
                       entity.update(app, |view, cx| {
                                 if view.panel_lookup_matches(id, &input, cx).is_some() {
                                     view.dismiss_panel_lookup(id, &input, cx);
                                     cx.stop_propagation();
                                     cx.notify();
                                 }
                             });
                   }
               })
               .capture_action::<IndentInline>({
                   let (entity, input) = (entity.clone(), input.clone());
                   move |_, window, app| {
                       entity.update(app, |view, cx| {
                                 if view.panel_lookup_matches(id, &input, cx).is_some() {
                                     view.insert_panel_lookup_entry(id, &input, window, cx);
                                     cx.stop_propagation();
                                 }
                             });
                   }
               })
               .capture_action::<Enter>({
                   let input = input.clone();
                   move |_, window, app| {
                       entity.update(app, |view, cx| {
                                 if view.panel_lookup_matches(id, &input, cx).is_some() {
                                     view.insert_panel_lookup_entry(id, &input, window, cx);
                                     cx.stop_propagation();
                                 }
                             });
                   }
               })
    }
}

/// Writes `/<token>` over the buffer span `range` holds, leaving the caret
/// just after it and the rest of the buffer untouched.
///
/// This is what makes the insertion surgical. The textarea exposes no
/// single replace-a-range call, but selecting the span and replacing the
/// selection does exactly that, and `replace` documents the caret landing
/// at the end of what it wrote - so the two calls together are the spec's
/// "token is replaced in place" with no whole-buffer fallback needed.
pub(crate) fn replace_lookup_token(input: &Entity<PanelInputState>,
                                   range: std::ops::Range<usize>, trigger: Trigger, token: &str,
                                   window: &mut Window, cx: &mut gpui_kit::App) {
    // The trigger comes from the token being replaced, not from the entry:
    // an `@` completion writes an `@` back, and a `/` completion a `/`.
    //
    // A mention is escaped so a path containing a space stays one token.
    // A command token has no whitespace to protect, and escaping it would
    // only make the buffer harder to read.
    let text = match trigger {
        Trigger::Slash => format!("/{token}"),
        Trigger::Mention => format!("@{}", escape_token(token)),
    };
    input.update(cx, |state, cx| {
             state.set_selected_range(range, cx);
             state.replace(&text, window, cx);
         });
}

/// Tints the selected row, so the popup's own styling stays out of the
/// render chain above.
trait SelectedRow {
    fn when_selected(self, selected: bool, cx: &Context<WorkspaceWindow>) -> Self;
}

impl SelectedRow for gpui_kit::Stateful<gpui_kit::Div> {
    fn when_selected(self, selected: bool, cx: &Context<WorkspaceWindow>) -> Self {
        if selected {
            self.bg(cx.theme().accent)
                .text_color(cx.theme().accent_foreground)
        }
        else {
            self
        }
    }
}
