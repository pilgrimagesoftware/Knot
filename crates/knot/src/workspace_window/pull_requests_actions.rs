//! The Pull Requests view's toolbar state and the list-level actions: the
//! search field, the filters and sort the user has chosen, Refresh now, Copy
//! URLs and the bulk removals.
//!
//! Contract: "The view's search, filters and sort belong to the window" and
//! "The user can act on the whole list" in the `pull-request-tracking`
//! capability spec under `openspec/specs/`.
//!
//! Which rows pass is `crate::pull_request_filter`, pure; this is the part
//! that needs the window - the search field's entity, the clipboard, the
//! confirmation dialog, the cache.

use std::collections::BTreeSet;

use gpui_kit::component::WindowExt;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::{App, AppContext, ClipboardItem, Context, Entity, Subscription, Window};
use uuid::Uuid;

use super::WorkspaceWindow;
use crate::pull_request_filter::{
    self, PullRequestGroup, PullRequestSort, RowCategory, ViewFilter,
};

/// What the user has chosen in the Pull Requests view's toolbar, held for as
/// long as the window is open and never persisted.
#[derive(Default)]
pub(crate) struct PullRequestViewState {
    pub(crate) sort:          PullRequestSort,
    /// The selected status toggles. Empty shows every status.
    pub(crate) statuses:      BTreeSet<RowCategory>,
    pub(crate) agent:         Option<Uuid>,
    /// Created on the view's first frame, since an input needs a `Window`.
    /// The text lives only here, so there is one source of truth for it.
    search:                   Option<Entity<InputState>>,
    /// Held for the entity's life: dropping it unsubscribes, and the list
    /// would stop narrowing as the user types.
    _search_changes:          Option<Subscription>,
    /// URLs Refresh now asked about again although their last answer was
    /// final (not found). Consumed one by one as each refresh is claimed, so
    /// a not-found row is asked once more and not on every later cycle.
    pub(crate) refresh_final: BTreeSet<String>,
}

/// The rows a bulk action was chosen over, captured when it was chosen.
pub(crate) struct BulkRemoval {
    pub(crate) rows:     Vec<(Vec<Uuid>, String)>,
    /// Whether a search or filter narrowed them, so the prompt says only the
    /// shown rows go.
    pub(crate) filtered: bool,
}

impl WorkspaceWindow {
    /// The search field, created on the first frame that needs it.
    pub(super) fn pull_request_search(&mut self, window: &mut Window, cx: &mut Context<Self>)
                                      -> Entity<InputState> {
        if let Some(input) = &self.pull_request_view.search {
            return input.clone();
        }
        let input = cx.new(|cx| {
                          InputState::new(window, cx)
                .placeholder(knot_core::l10n::t("pull_requests.search_placeholder"))
                .clean_on_escape()
                      });
        // Every way text changes reports here, so the list narrows per
        // keystroke without a submit.
        let subscription = cx.subscribe_in(&input, window, |_: &mut Self, _, event, _, cx| {
                                 if matches!(event, InputEvent::Change) {
                                     cx.notify();
                                 }
                             });
        self.pull_request_view.search = Some(input.clone());
        self.pull_request_view._search_changes = Some(subscription);
        input
    }

    /// The search, filters and agent as they stand this frame.
    pub(super) fn pull_request_filter(&self, cx: &App) -> ViewFilter {
        let search = self.pull_request_view
                         .search
                         .as_ref()
                         .map(|input| input.read(cx).value().to_string())
                         .unwrap_or_default();
        ViewFilter { search,
                     statuses: self.pull_request_view.statuses.clone(),
                     agent: self.pull_request_view.agent }
    }

    /// Every group, and the groups the view draws.
    ///
    /// An agent filter naming no agent the list still has - removed, or its
    /// last row expired - goes back to all agents here, rather than leaving
    /// the view empty for a reason the user can no longer see.
    pub(super) fn filtered_pull_request_groups(
        &mut self, cx: &App)
        -> (Vec<PullRequestGroup>, Vec<PullRequestGroup>) {
        let groups = self.pull_request_groups();
        if let Some(agent) = self.pull_request_view.agent
           && !pull_request_filter::listed_agents(&groups).contains(&agent)
        {
            self.pull_request_view.agent = None;
        }
        let filter = self.pull_request_filter(cx);
        let shown =
            pull_request_filter::apply(groups.clone(), &filter, self.pull_request_view.sort);
        (groups, shown)
    }

    /// Empty the search, deselect every toggle and return to all agents. The
    /// sort order stays: it is a preference, not a narrowing.
    pub(super) fn clear_pull_request_filters(&mut self, window: &mut Window,
                                             cx: &mut Context<Self>) {
        if let Some(input) = &self.pull_request_view.search {
            input.update(cx, |input, cx| input.set_value("", window, cx));
        }
        self.pull_request_view.statuses.clear();
        self.pull_request_view.agent = None;
        cx.notify();
    }

    /// Select or deselect one status toggle.
    pub(super) fn toggle_pull_request_status(&mut self, category: RowCategory) {
        let statuses = &mut self.pull_request_view.statuses;
        if !statuses.remove(&category) {
            statuses.insert(category);
        }
    }

    /// Fetch every listed pull request again now, hidden and not-found rows
    /// included, and ask `gh` again whether it is usable.
    ///
    /// Marks the cache stale rather than forgetting it, so every row keeps
    /// the state it has until its new answer lands. The fetches themselves
    /// are claimed by the next frame's `refresh_pull_request_states`, which
    /// the caller's `notify` schedules; their answers reach a frame through
    /// the cache's existing `take_changed` link in `repaint_poll_tick`.
    pub(super) fn refresh_pull_requests_now(&mut self) {
        self.pull_request_states.mark_all_stale();
        self.forge_status.mark_stale();
        self.pull_request_view.refresh_final =
            self.workspace_pull_request_urls().into_iter().collect();
    }

    /// Put `text` on the clipboard.
    pub(super) fn copy_to_clipboard(text: String, cx: &mut App) {
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }

    /// Ask before removing several rows, then remove them in one pass.
    ///
    /// The rows were captured when the action was chosen, so what the prompt
    /// counts is what goes - not whatever the list holds when OK is pressed.
    pub(super) fn confirm_bulk_remove(&mut self, removal: BulkRemoval, window: &mut Window,
                                      cx: &mut Context<Self>) {
        if removal.rows.is_empty() {
            return;
        }
        let count = removal.rows.len().to_string();
        let mut body =
            knot_core::l10n::t_with("pull_requests.bulk_remove_body", &[("count", &count)]);
        if removal.filtered {
            body.push(' ');
            body.push_str(&knot_core::l10n::t("pull_requests.bulk_remove_shown"));
        }
        let entity = cx.entity();
        let rows = removal.rows;
        window.open_alert_dialog(cx, move |alert, _, _| {
                  let entity = entity.clone();
                  let rows = rows.clone();
                  alert.title(knot_core::l10n::t("pull_requests.bulk_remove_title"))
                       .description(body.clone())
                       .confirm()
                       .on_ok(move |_, _, app| {
                           entity.update(app, |view, cx| {
                                     view.remove_pull_requests(&rows, cx);
                                     cx.notify();
                                 });
                           true
                       })
              });
    }
}
