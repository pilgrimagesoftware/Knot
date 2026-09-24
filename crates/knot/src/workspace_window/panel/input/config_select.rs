//! The searchable, scrolling dropdown behind the model and effort
//! selectors, and the per-panel state those need to keep between frames.
//!
//! Contract: `openspec/specs/acp-panel-ui/spec.md` - "Model dropdown
//! scrolls" and "Model dropdown searches". The model list is whatever the
//! adapter declared, which is routinely longer than a popover can show, so
//! the list a user cannot scroll is a list whose tail is unreachable.
//!
//! The permission selector deliberately does *not* come through here - see
//! `controls.rs`: `SelectState` keeps its open flag private, and ⌘⇧P opens
//! the permission menu from a key binding, which only the hand-rolled
//! popover can do.

use gpui_kit::App;
use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::Entity;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::SharedString;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::component::IndexPath;
use gpui_kit::component::Sizable;
use gpui_kit::component::searchable_list::SearchableListItem;
use gpui_kit::component::searchable_list::SearchableVec;
use gpui_kit::component::select::Select;
use gpui_kit::component::select::SelectEvent;
use gpui_kit::component::select::SelectState;
use gpui_kit::div;
use uuid::Uuid;

use crate::panel_session;
use crate::workspace_window::WorkspaceWindow;

/// How tall the dropdown may grow before it scrolls, in rems. The kit's own
/// default is the same 20rem; naming it here is what the "more models than
/// fit" scenario measures against.
pub(crate) const MENU_MAX_HEIGHT_REMS: f32 = 20.;

pub(in crate::workspace_window) const MODEL_SELECTOR_ID: &str = "panel-model-selector";

pub(in crate::workspace_window) const EFFORT_SELECTOR_ID: &str = "panel-effort-selector";

/// The selectors drawn as a searchable, scrolling `Select`, with the
/// category names each one's option is found under.
///
/// The permission selector is absent on purpose: ⌘⇧P opens its menu from a
/// key binding, and the kit's `SelectState` keeps its open flag private, so
/// only the hand-rolled popover in `controls` can be opened from an action.
/// Its list is a handful of modes, which is the case that popover was
/// already adequate for.
pub(in crate::workspace_window) const SEARCHABLE_SELECTORS: &[(&str, &[&str])] =
    &[(MODEL_SELECTOR_ID, &["model"]),
      (EFFORT_SELECTOR_ID,
       &["effort",
         "reasoning",
         "reasoning_effort",
         "reasoning-effort",
         "thought_level",
         "thought-level"])];

/// One declared value of a Session Config Option, as the dropdown lists it.
///
/// `title` is the name the agent gave the value, which is both what is drawn
/// and what the search filters on - `SearchableListItem::matches` defaults to
/// a case-insensitive substring of the title, which is exactly the rule the
/// spec's "Typing narrows the list" scenario states.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ConfigSelectorItem {
    /// What `session/set_config_option` is given.
    value: String,
    /// What the user reads and searches.
    name:  String,
}

impl SearchableListItem for ConfigSelectorItem {
    type Value = String;

    fn title(&self) -> SharedString {
        self.name.clone().into()
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }
}

/// The dropdown's item source. `SearchableVec` is what filters: a plain
/// `Vec` delegate ignores the query entirely, so the search field would draw
/// and do nothing.
pub(crate) type ConfigSelectorDelegate = SearchableVec<ConfigSelectorItem>;

/// Identifies one panel's one selector. The element id is the selector's
/// axis (`panel-model-selector`, `panel-effort-selector`), so a window
/// holding several panels keeps their model dropdowns - and each panel's
/// model and effort dropdowns - apart.
pub(in crate::workspace_window) type SelectorKey = (Uuid, &'static str);

/// The values, in declared order, that a dropdown for `option` lists.
pub(crate) fn config_selector_items(option: &knot_acp::ConfigOption) -> Vec<ConfigSelectorItem> {
    option.options
          .iter()
          .map(|value| ConfigSelectorItem { value: value.value.clone(),
                                            name:  value.name.clone(), })
          .collect()
}

/// The dropdown state for one declared option, selection parked on the
/// value the agent reports as current.
///
/// The one constructor: the window builds its dropdowns through this and so
/// do the tests, so what is tested is what is drawn.
pub(crate) fn new_config_select_state(option: &knot_acp::ConfigOption, window: &mut Window,
                                      cx: &mut App)
                                      -> Entity<SelectState<ConfigSelectorDelegate>> {
    let items = config_selector_items(option);
    let current = option.current_value.as_str().unwrap_or_default();
    let selected = items.iter()
                        .position(|item| item.value == current)
                        .map(IndexPath::new);
    let delegate = ConfigSelectorDelegate::new(items);
    cx.new(|cx| SelectState::new(delegate, selected, window, cx).searchable(true))
}

impl WorkspaceWindow {
    /// Makes sure `id`'s model and effort selectors have their `SelectState`,
    /// and that what it lists is what the agent currently declares.
    ///
    /// Called from `prepare_frame` rather than from the render path: the
    /// state holds focus, scroll offset and the search query, so rebuilding
    /// it per frame would drop a keystroke the moment it was typed. Building
    /// it also needs `&mut Window`, which the render path does not carry.
    ///
    /// Adapters re-report their options mid-session (a model list arrives
    /// after the agent connects, and can change again), so a state built
    /// once and never refreshed is a dropdown listing models the agent no
    /// longer offers. The item list is compared before it is replaced,
    /// which is what keeps an unchanged option from clearing the query the
    /// user is part-way through typing.
    pub(in crate::workspace_window) fn ensure_panel_config_selectors(&mut self, id: Uuid,
                                                                     options: &[knot_acp::ConfigOption],
                                                                     window: &mut Window,
                                                                     cx: &mut Context<Self>) {
        for (element_id, categories) in super::SEARCHABLE_SELECTORS {
            let Some(option) = Self::find_config_option(options, categories)
            else {
                continue;
            };
            let key = (id, *element_id);
            let items = config_selector_items(option);
            let current = option.current_value
                                .as_str()
                                .unwrap_or_default()
                                .to_string();
            if let Some(state) = self.panel_config_selectors.get(&key).cloned() {
                if self.panel_config_selector_items.get(&key) == Some(&items) {
                    continue;
                }
                state.update(cx, |state, cx| {
                         state.set_items(ConfigSelectorDelegate::new(items.clone()), window, cx);
                         state.set_selected_value(&current, window, cx);
                     });
                self.panel_config_selector_items.insert(key, items);
                continue;
            }
            let state = new_config_select_state(option, window, cx);
            let config_id = option.id.clone();
            let subscription =
                cx.subscribe(&state, move |view, _state, event, cx| {
                      let SelectEvent::Confirm(Some(value)) = event
                      else {
                          return;
                      };
                      view.apply_panel_config_selection(id, config_id.clone(), value.clone(), cx);
                  });
            self.panel_config_selectors.insert(key, state);
            self.panel_config_selector_subscriptions
                .insert(key, subscription);
            self.panel_config_selector_items.insert(key, items);
        }
    }

    /// Persists a chosen value and applies it to the live session - the same
    /// two steps the permission selector's click path runs, per
    /// `acp-panel-ui`'s selector requirements: durable whether or not a
    /// session is up, and in force for the next turn when one is.
    pub(crate) fn apply_panel_config_selection(&mut self, id: Uuid, config_id: String,
                                               value: String, cx: &mut Context<Self>) {
        // Persist first: the selection is durable whether or not a session
        // is live to apply it to.
        self.remember_session_config(id, config_id.clone(), value.clone(), cx);
        let Some(session_arc) = self.panel_sessions.get(&id).cloned()
        else {
            return;
        };
        if let panel_session::PanelSessionSlot::Ready(handle) = &*session_arc.lock() {
            let future = handle.set_config_option(config_id, value);
            let _guard = self.runtime.enter();
            self.runtime.spawn(future);
        }
    }

    /// The dropdown itself, or `None` before `ensure_panel_config_selectors`
    /// has built its state - one frame at most, and an axis the agent never
    /// declared never gets one at all.
    pub(in crate::workspace_window) fn render_panel_config_select(
        &self, id: Uuid, element_id: &'static str)
        -> Option<impl IntoElement + use<>> {
        let state = self.panel_config_selectors.get(&(id, element_id))?;
        Some(Select::new(state).id(gpui_kit::ElementId::from(format!("{element_id}-{id}")))
                               .small()
                               .appearance(false)
                               .menu_max_h(gpui_kit::rems(MENU_MAX_HEIGHT_REMS))
                               .search_placeholder(knot_core::l10n::t("panel.selector.search_placeholder"))
                               .empty(|_window, _app| {
                                   div().p_1()
                                        .child(knot_core::l10n::t("panel.selector.no_matches"))
                               }))
    }

    /// Drops every selector `id` owns. Called from the session teardown that
    /// already prunes the rest of the per-agent view state: these are keyed
    /// by agent id and written from a frame, so an agent that is gone would
    /// otherwise keep its dropdowns - and their subscriptions - for the
    /// window's whole life.
    pub(in crate::workspace_window) fn forget_panel_config_selectors(&mut self, id: Uuid) {
        self.panel_config_selectors
            .retain(|(agent, _), _| *agent != id);
        self.panel_config_selector_subscriptions
            .retain(|(agent, _), _| *agent != id);
        self.panel_config_selector_items
            .retain(|(agent, _), _| *agent != id);
    }
}
