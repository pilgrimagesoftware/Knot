use super::*;

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
    std::fs::write(path, text).expect("write");
}

#[test]
fn a_file_that_sets_default_mode_is_a_choice() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = dir.path().join("settings.json");
    write(&file, r#"{"permissions": {"defaultMode": "manual"}}"#);
    assert!(sets_default_mode(&file));
}

#[test]
fn other_permissions_are_not_a_mode() {
    let dir = tempfile::tempdir().expect("temp dir");
    let file = dir.path().join("settings.json");
    write(&file, r#"{"permissions": {"allow": ["Bash(ls)"]}}"#);
    assert!(!sets_default_mode(&file));
}

#[test]
fn a_missing_or_broken_file_sets_nothing() {
    let dir = tempfile::tempdir().expect("temp dir");
    assert!(!sets_default_mode(&dir.path().join("absent.json")));
    let broken = dir.path().join("broken.json");
    write(&broken, "{ not json");
    assert!(!sets_default_mode(&broken));
}

#[test]
fn the_project_and_user_files_are_both_read() {
    let files = claude_settings_files(Path::new("/work/repo"), Some(Path::new("/home/me/.claude")));
    assert!(files.contains(&PathBuf::from("/work/repo/.claude/settings.local.json")));
    assert!(files.contains(&PathBuf::from("/work/repo/.claude/settings.json")));
    assert!(files.contains(&PathBuf::from("/home/me/.claude/settings.json")));
}

#[test]
fn a_project_default_mode_leaves_the_adapter_alone() {
    let dir = tempfile::tempdir().expect("temp dir");
    write(&dir.path().join(".claude/settings.json"),
          r#"{"permissions": {"defaultMode": "plan"}}"#);
    assert_eq!(default_mode_with("claude", dir.path(), None), None);
}

#[test]
fn a_user_default_mode_leaves_the_adapter_alone() {
    let project = tempfile::tempdir().expect("temp dir");
    let config = tempfile::tempdir().expect("temp dir");
    write(&config.path().join("settings.json"),
          r#"{"permissions": {"defaultMode": "acceptEdits"}}"#);
    assert_eq!(default_mode_with("claude", project.path(), Some(config.path())),
               None);
}

/// Managed settings live at a fixed system path this test cannot stub, so it
/// only asserts where that file does not exist - which is the usual case.
#[test]
fn with_nothing_configured_claude_starts_in_auto() {
    if Path::new(CLAUDE_MANAGED_SETTINGS_MACOS).exists() {
        return;
    }
    let project = tempfile::tempdir().expect("temp dir");
    let config = tempfile::tempdir().expect("temp dir");
    write(&config.path().join("settings.json"), r#"{"model": "opus"}"#);
    assert_eq!(default_mode_with("claude", project.path(), Some(config.path())),
               Some(DefaultMode { config_id: "mode",
                                  value:     "auto", }));
}

#[test]
fn only_claude_has_a_differing_default() {
    let dir = tempfile::tempdir().expect("temp dir");
    for agent_type in ["codex", "gemini", "opencode", "copilot", "shell"] {
        assert_eq!(default_mode_with(agent_type, dir.path(), None),
                   None,
                   "{agent_type}");
    }
}
