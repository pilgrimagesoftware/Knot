//! The menu bar's Agents menu: its pairing with the context menu's item
//! set, and the shape it is built into. Contract: `openspec/specs/app-menu`.

use std::path::PathBuf;

use gpui_kit::MenuItem;
use uuid::Uuid;

use crate::agent_menu::AgentMenuSnapshot;
use crate::agent_menu::agent_menu_action;
use crate::agent_menu::agent_menu_entry_for_action;
use crate::agent_menu::agents_menu;
use crate::app_state::AgentMenuEntry;
use crate::app_state::AgentMenuFacts;
use crate::app_state::agent_context_menu_entries;
use crate::tests::menu_labels;

/// Exhaustive by construction: a new `AgentMenuEntry` variant makes this
/// match fail to compile, which is what stops an item being added to the
/// context menu without anyone noticing it never reached the menu bar.
fn entry_ordinal(entry: AgentMenuEntry) -> usize {
    match entry {
        AgentMenuEntry::Separator => 0,
        AgentMenuEntry::NewCompanion => 1,
        AgentMenuEntry::NewShellCompanion => 2,
        AgentMenuEntry::EditAgent => 3,
        AgentMenuEntry::ForkAgent => 4,
        AgentMenuEntry::DuplicateAgent => 5,
        AgentMenuEntry::MoveToWorkspace => 6,
        AgentMenuEntry::SaveToBench => 7,
        AgentMenuEntry::OpenIn => 8,
        AgentMenuEntry::MarkdownFiles => 9,
        AgentMenuEntry::RegisterAgent => 10,
        AgentMenuEntry::Deactivate => 11,
        AgentMenuEntry::RestartAgent => 12,
        AgentMenuEntry::RemoveAgent => 13,
    }
}

#[test]
fn every_agent_menu_entry_is_in_all() {
    let mut ordinals = AgentMenuEntry::ALL.into_iter()
                                          .map(entry_ordinal)
                                          .collect::<Vec<_>>();
    ordinals.sort_unstable();
    assert_eq!(ordinals,
               (0..AgentMenuEntry::ALL.len()).collect::<Vec<_>>(),
               "`ALL` must list every variant exactly once");
}

/// Both menus are built from one item set, so an entry with a label must
/// have an action to dispatch, and an action must name exactly one entry.
/// Without this pairing an item could reach one menu and skip the other.
#[test]
fn every_labelled_entry_has_exactly_one_menu_action() {
    for entry in AgentMenuEntry::ALL {
        let action = agent_menu_action(entry);
        assert_eq!(action.is_some(),
                   entry.label().is_some(),
                   "{entry:?} must have an action if and only if it has a label");
        let Some(action) = action
        else {
            continue;
        };
        assert_eq!(agent_menu_entry_for_action(action.as_ref()),
                   Some(entry),
                   "{entry:?}'s action must map back to it alone");
    }
}

/// The label each item carries, separators rendered as `"-"`, so ordering
/// *and* divider placement are both asserted - the same shape
/// `tests::menu_labels` produces for the context menu.
fn agents_menu_labels(snapshot: &AgentMenuSnapshot) -> Vec<String> {
    agents_menu(snapshot).items
                         .iter()
                         .map(|item| match item {
                             MenuItem::Separator => "-".to_string(),
                             MenuItem::Action { name, .. } => name.to_string(),
                             MenuItem::Submenu(submenu) => submenu.name.to_string(),
                             MenuItem::SystemMenu(_) => unreachable!("no OS submenu here"),
                         })
                         .collect()
}

/// The Agents menu's shape is the context menu's, taken from the same
/// function: same items, same order, same dividers.
#[test]
fn the_agents_menu_lists_the_context_menus_items_in_order() {
    assert_eq!(agents_menu_labels(&AgentMenuSnapshot::default()),
               menu_labels(AgentMenuFacts::EVERY_ITEM));
}

/// The menu bar's deliberate divergence from the context menu: an item the
/// selected agent cannot use holds its position rather than disappearing,
/// so the menu can be navigated from memory. Which items are *enabled* is
/// decided per item by macOS asking whether the action is available, not
/// by this list.
#[test]
fn the_agents_menu_keeps_items_a_companion_cannot_use() {
    let labels = agents_menu_labels(&AgentMenuSnapshot::default());
    let companion = menu_labels(AgentMenuFacts { is_companion: true,
                                                 is_shell: true,
                                                 ..AgentMenuFacts::EVERY_ITEM });

    for entry in [AgentMenuEntry::ForkAgent,
                  AgentMenuEntry::DuplicateAgent,
                  AgentMenuEntry::RegisterAgent]
    {
        let hidden = entry.label().expect("a labelled entry");
        assert!(!companion.contains(&hidden),
                "a companion's context menu omits {entry:?}");
        assert!(labels.contains(&hidden), "the Agents menu keeps {entry:?}");
    }
}

/// The dynamic submenus are built from the snapshot, which is what lets a
/// workspace created - or a markdown file shown - since the menu bar was
/// last built appear in them without relaunching.
#[test]
fn the_agents_menu_submenus_come_from_the_snapshot() {
    let snapshot = AgentMenuSnapshot { entries:
                                           agent_context_menu_entries(AgentMenuFacts::EVERY_ITEM),
                                       move_targets:     vec![(Uuid::new_v4(),
                                                               "Second".to_string())],
                                       markdown_history: vec![PathBuf::from("/tmp/notes/plan.md")], };

    let move_targets = submenu_items(&snapshot, &entry_label(AgentMenuEntry::MoveToWorkspace));
    assert_eq!(move_targets, vec!["Second".to_string()]);

    // The file name, not the path - a full path makes the submenu
    // unreadable, and the context menu labels these the same way.
    assert_eq!(submenu_items(&snapshot, &entry_label(AgentMenuEntry::MarkdownFiles)),
               vec!["plan.md".to_string()]);

    // Open In… is a fixed list, so it needs nothing from the snapshot.
    assert!(submenu_items(&snapshot, &entry_label(AgentMenuEntry::OpenIn)).contains(&"Finder".to_string()));
}

/// With no agent selected the submenus keep their positions, empty and
/// disabled.
///
/// They have to be disabled explicitly: AppKit only validates items that
/// carry an action, and a submenu's parent carries none, so an unavailable
/// submenu draws enabled unless this says otherwise.
#[test]
fn the_agents_menu_disables_its_submenus_with_nothing_selected() {
    let empty = AgentMenuSnapshot::default();
    let labels = agents_menu_labels(&empty);
    for title in [AgentMenuEntry::MoveToWorkspace,
                  AgentMenuEntry::OpenIn,
                  AgentMenuEntry::MarkdownFiles].map(entry_label)
    {
        assert!(labels.contains(&title), "{title} should still be present");
        assert!(submenu(&empty, &title).disabled,
                "{title} should be disabled");
    }
    assert!(submenu_items(&empty, &entry_label(AgentMenuEntry::MoveToWorkspace)).is_empty());
    assert!(submenu_items(&empty, &entry_label(AgentMenuEntry::MarkdownFiles)).is_empty());
}

/// A submenu the selected agent cannot use is disabled too - a companion
/// cannot be moved, and an agent that has shown no markdown file has no
/// files to list - while the one it can use stays enabled.
#[test]
fn the_agents_menu_disables_the_submenus_a_companion_cannot_use() {
    let facts = AgentMenuFacts { is_companion: true,
                                 is_shell: true,
                                 has_markdown_history: false,
                                 ..AgentMenuFacts::EVERY_ITEM };
    let snapshot = AgentMenuSnapshot { entries: agent_context_menu_entries(facts),
                                       ..AgentMenuSnapshot::default() };

    assert!(submenu(&snapshot, &entry_label(AgentMenuEntry::MoveToWorkspace)).disabled);
    assert!(submenu(&snapshot, &entry_label(AgentMenuEntry::MarkdownFiles)).disabled);
    assert!(!submenu(&snapshot, &entry_label(AgentMenuEntry::OpenIn)).disabled,
            "Open In… applies to every agent");
}

/// The menu title an entry carries, for finding the submenu it opens. Taken
/// from the entry rather than written out, so these tests name the item and
/// not its English copy.
fn entry_label(entry: AgentMenuEntry) -> String {
    entry.label().expect("a labelled entry")
}

/// The submenu titled `title`.
fn submenu(snapshot: &AgentMenuSnapshot, title: &str) -> gpui_kit::Menu {
    agents_menu(snapshot).items
                         .into_iter()
                         .find_map(|item| match item {
                             MenuItem::Submenu(submenu) if submenu.name == title => Some(submenu),
                             _ => None,
                         })
                         .unwrap_or_else(|| panic!("{title} should be a submenu"))
}

/// The labels inside the submenu titled `title`, separators dropped.
fn submenu_items(snapshot: &AgentMenuSnapshot, title: &str) -> Vec<String> {
    submenu(snapshot, title).items
                            .into_iter()
                            .filter_map(|item| match item {
                                MenuItem::Action { name, .. } => Some(name.to_string()),
                                _ => None,
                            })
                            .collect()
}
