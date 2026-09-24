//! The model selector's dropdown: that a long list keeps every model and is
//! bounded rather than run off the window, and that the search field's
//! filter is a case-insensitive substring of the model's name
//! (`openspec/specs/acp-panel-ui`, "Model dropdown scrolls" and "Model
//! dropdown searches").
//!
//! These build the dropdown through `new_config_select_state`, the same
//! constructor `prepare_frame` builds the real one with, in a real window -
//! `SelectState` wants a `Window` and its list measures itself, so a
//! hand-built stand-in would be answering a different question.
//!
//! What they do *not* reach: the kit owns the scroll container and the
//! search input, and keeps both private to its own crate, so there is no
//! way from here to type into the field or read a scroll offset back. The
//! filter is therefore driven through `perform_search` - the same call the
//! search field makes on each keystroke - and the height bound is asserted
//! as the cap the dropdown is built with against the height the rows would
//! otherwise take. The remaining gap (that the kit's list actually scrolls
//! within that cap) is the kit's own contract, not this crate's.

use std::sync::Arc;

use gpui_kit::TestAppContext;
use gpui_kit::component::IndexPath;
use gpui_kit::component::searchable_list::SearchableListDelegate;
use gpui_kit::component::searchable_list::SearchableListItem;
use parking_lot::Mutex;
use tempfile::TempDir;

use crate::settings_global;
use crate::window_registry::WindowKey;
use crate::window_registry::WindowRegistry;
use crate::workspace_window::WorkspaceWindow;
use crate::workspace_window::panel::input::ConfigSelectorDelegate;
use crate::workspace_window::panel::input::MENU_MAX_HEIGHT_REMS;
use crate::workspace_window::panel::input::config_selector_items;
use crate::workspace_window::panel::input::new_config_select_state;

/// More models than any dropdown this tall can show at once - the case the
/// requirement exists for.
const LONG_LIST: usize = 40;

/// A row in the kit's list is one line of text plus its padding. Only used
/// to say "these do not all fit", so it is deliberately generous: if a row
/// were really this tall, fewer would fit, not more.
const ROW_HEIGHT_REMS: f32 = 2.;

/// The declared `model` option an agent reports, listing `names`.
fn model_option(names: &[&str], current: &str) -> knot_acp::ConfigOption {
    knot_acp::ConfigOption { id:            "model".to_string(),
                             name:          "Model".to_string(),
                             category:      Some("model".to_string()),
                             kind:          "select".to_string(),
                             current_value: serde_json::Value::String(current.to_string()),
                             options:       names.iter()
                                                 .map(|name| {
                                                     knot_acp::ConfigOptionValue {
                                                         value:       name.to_lowercase()
                                                                          .replace(' ', "-"),
                                                         name:        (*name).to_string(),
                                                         description: None,
                                                     }
                                                 })
                                                 .collect(), }
}

/// `LONG_LIST` models, named so that a substring matches a known few.
fn many_models() -> Vec<String> {
    (0..LONG_LIST).map(|index| match index % 4 {
                      0 => format!("Claude Opus {index}"),
                      1 => format!("Claude Sonnet {index}"),
                      2 => format!("GPT {index}"),
                      _ => format!("Gemini {index}"),
                  })
                  .collect()
}

/// The titles a delegate currently offers, in order.
fn listed(delegate: &ConfigSelectorDelegate) -> Vec<String> {
    (0..delegate.items_count(0)).filter_map(|row| delegate.item(IndexPath::new(row)))
                                .map(|item| item.title().to_string())
                                .collect()
}

/// A delegate over `names`, filtered by `query` the way the search field
/// filters it on each keystroke.
fn searched(cx: &mut TestAppContext, names: &[&str], query: &str) -> Vec<String> {
    let window = cx.add_empty_window();
    window.update(|window, cx| {
              let mut delegate =
                  ConfigSelectorDelegate::new(config_selector_items(&model_option(names, "")));
              // The task is ready immediately for an in-memory list; the
              // filtered view is rebuilt before it returns.
              drop(delegate.perform_search(query, window, cx));
              listed(&delegate)
          })
}

/// The dropdown cannot be allowed to grow with the list: the popover it
/// replaced had no bound at all, which is how a long model list ran off the
/// window edge and left its tail unreachable.
#[gpui_kit::test]
fn a_long_model_list_is_capped_below_the_height_its_rows_would_take(cx: &mut TestAppContext) {
    let models = many_models();
    let names: Vec<&str> = models.iter().map(String::as_str).collect();
    let window = cx.add_empty_window();

    let offered = window.update(|window, cx| {
                            let option = model_option(&names, "");
                            // Built, so a cap that broke the constructor
                            // would fail here rather than on screen.
                            let _state = new_config_select_state(&option, window, cx);
                            config_selector_items(&option).len()
                        });

    assert_eq!(offered, LONG_LIST,
               "the cap bounds the height, never the list");
    assert!(MENU_MAX_HEIGHT_REMS < LONG_LIST as f32 * ROW_HEIGHT_REMS,
            "{LONG_LIST} rows must not fit in {MENU_MAX_HEIGHT_REMS}rem - otherwise the \
             scrolling requirement is being tested against a list that never needed it");
}

/// Capping the height must not cost a model: what is offered is still every
/// model the agent declared, in the order it declared them.
#[gpui_kit::test]
fn every_declared_model_is_offered_in_the_agents_order(cx: &mut TestAppContext) {
    let models = many_models();
    let names: Vec<&str> = models.iter().map(String::as_str).collect();

    let listed_models = searched(cx, &names, "");

    assert_eq!(listed_models, names,
               "an empty query offers every model, in declared order");
}

/// A short list is the other half of the requirement: it is shown whole, and
/// nothing about the cap applies to it.
#[gpui_kit::test]
fn a_short_model_list_is_offered_whole(cx: &mut TestAppContext) {
    let names = ["Claude Opus", "Claude Sonnet", "Claude Haiku"];

    let listed_models = searched(cx, &names, "");

    assert_eq!(listed_models, names);
    assert!(names.len() as f32 * ROW_HEIGHT_REMS < MENU_MAX_HEIGHT_REMS,
            "three rows fit inside the cap, so this list never scrolls");
}

/// The search itself, and the case-insensitivity the requirement states.
#[gpui_kit::test]
fn typing_narrows_the_list_to_substring_matches(cx: &mut TestAppContext) {
    let names = ["Claude Opus", "Claude Sonnet", "GPT-5", "Gemini Pro"];

    assert_eq!(searched(cx, &names, "claude"),
               ["Claude Opus", "Claude Sonnet"],
               "a lowercase query must match names that are not lowercase");
    assert_eq!(searched(cx, &names, "OPUS"),
               ["Claude Opus"],
               "an uppercase query must match a name that is not uppercase");
    assert_eq!(searched(cx, &names, "e"),
               ["Claude Opus", "Claude Sonnet", "Gemini Pro"],
               "the match is a substring, not a prefix");
}

/// "No match selects nothing": the dropdown offers no model rather than
/// falling back to the whole list, which would hand the user a model they
/// did not search for.
#[gpui_kit::test]
fn a_query_matching_no_model_offers_none(cx: &mut TestAppContext) {
    let names = ["Claude Opus", "GPT-5"];

    assert!(searched(cx, &names, "llama").is_empty());
}

/// Clearing the field restores the full list - the way back from a search
/// that found nothing.
#[gpui_kit::test]
fn clearing_the_query_restores_every_model(cx: &mut TestAppContext) {
    let names = ["Claude Opus", "Claude Sonnet", "GPT-5"];
    let window = cx.add_empty_window();

    let restored = window.update(|window, cx| {
                             let mut delegate =
            ConfigSelectorDelegate::new(config_selector_items(&model_option(&names, "")));
                             drop(delegate.perform_search("gpt", window, cx));
                             assert_eq!(listed(&delegate).len(), 1, "the query narrowed the list");
                             drop(delegate.perform_search("", window, cx));
                             listed(&delegate)
                         });

    assert_eq!(restored, names);
}

/// The value a model row carries is the one `session/set_config_option`
/// takes, not the name drawn beside it - a dropdown that sent the display
/// name would be rejected by the agent.
#[gpui_kit::test]
fn a_model_row_carries_the_declared_value_not_its_name(cx: &mut TestAppContext) {
    let window = cx.add_empty_window();

    let (title, value) = window.update(|_window, _cx| {
                                   let items =
                                       config_selector_items(&model_option(&["Claude Opus"], ""));
                                   let item = items.first().expect("one declared model").clone();
                                   (item.title().to_string(), item.value().clone())
                               });

    assert_eq!(title, "Claude Opus");
    assert_eq!(value, "claude-opus");
}

/// What a confirmed model does, which is the half of the dropdown that
/// outlives the frame: the choice is recorded against the agent, so the
/// next session replays it (`acp-panel-ui`'s selector requirements - the
/// selection is durable whether or not a session is live to apply it to).
///
/// Driven through `apply_panel_config_selection`, the body the
/// `SelectEvent::Confirm` subscription runs. The emit itself belongs to the
/// kit's own confirm path, which is private to it.
///
/// The settings here are rooted at a temporary directory: this reaches
/// `persist_agents`, and a `Settings::default()` writes over the user's own
/// roster.
#[gpui_kit::test]
fn confirming_a_model_records_it_against_the_agent(cx: &mut TestAppContext) {
    let mut store = knot_agents::AgentStore::new();
    let space = crate::tests::workspace("Only");
    let workspace_id = space.id;
    store.add_workspace(space);
    store.set_current_workspace(workspace_id);
    let agent = store.create("~/agent", knot_agents::CreateOptions::default());
    let store = Arc::new(Mutex::new(store));
    let messages = Arc::new(Mutex::new(knot_messaging::MessageStore::new()));
    let dir = TempDir::new().expect("a temporary settings root");

    cx.update(|cx| {
          gpui_kit::init(cx);
          WindowRegistry::install(cx);
          settings_global::install(knot_core::Settings::with_store_root(dir.path()), cx);
          WorkspaceWindow::open(Arc::clone(&store), Arc::clone(&messages), workspace_id, cx);
          let view = WindowRegistry::workspace_view(WindowKey::Workspace(workspace_id), cx)
            .expect("opening a workspace registers its view");

          assert!(store.lock().session_config(agent).is_empty(),
                  "nothing is recorded before a model is confirmed - otherwise this test \
                   cannot tell a working write from a default");

          view.update(cx, |view, cx| {
                  view.apply_panel_config_selection(agent,
                                                    "model".to_string(),
                                                    "claude-opus".to_string(),
                                                    cx);
              });

          assert_eq!(store.lock()
                          .session_config(agent)
                          .get("model")
                          .map(String::as_str),
                     Some("claude-opus"),
                     "the confirmed model is what the next session replays");
      });
}

/// The selection the dropdown opens on is the model the agent reports as
/// current, so opening it does not misreport what the next turn will run
/// under.
#[gpui_kit::test]
fn the_dropdown_opens_on_the_agents_current_model(cx: &mut TestAppContext) {
    let window = cx.add_empty_window();

    let selected = window.update(|window, cx| {
                             let option =
                                 model_option(&["Claude Opus", "Claude Sonnet"], "claude-sonnet");
                             let state = new_config_select_state(&option, window, cx);
                             state.read(cx).selected_value().cloned()
                         });

    assert_eq!(selected.as_deref(), Some("claude-sonnet"));
}
