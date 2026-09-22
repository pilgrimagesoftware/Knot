use std::path::Path;

use super::StorePaths;

#[test]
fn rooted_derives_every_document_from_one_directory() {
    let paths = StorePaths::rooted("/store");

    assert_eq!(paths.preferences(), Path::new("/store/preferences.json"));
    assert_eq!(paths.agents(), Path::new("/store/agents.json"));
    assert_eq!(paths.workspaces(), Path::new("/store/workspaces.json"));
    assert_eq!(paths.personas(), Path::new("/store/personas.json"));
    assert_eq!(paths.bench(), Path::new("/store/bench.json"));
    assert_eq!(paths.recent_repos(), Path::new("/store/recent-repos.json"));
    assert_eq!(paths.legacy(), Path::new("/store/settings.json"));
}

/// The point of the split: preferences belong with the platform's other
/// preferences, not inside the directory holding the user's agents and
/// workspaces. Only asserted where the platform actually separates the two -
/// on Linux and Windows the directories coincide by design.
#[test]
#[cfg(target_os = "macos")]
fn platform_preferences_live_outside_the_data_directory() {
    let Some(paths) = StorePaths::platform()
    else {
        return;
    };

    let preferences_dir = paths.preferences().parent().unwrap().to_path_buf();
    let data_dir = paths.agents().parent().unwrap().to_path_buf();
    assert_ne!(preferences_dir,
               data_dir,
               "the preferences document must not live in the data directory {}",
               data_dir.display());
}
