use super::log_dir;
use crate::consts::APP_NAME;

/// Asserts the shape of the path, never creating it: a test must not leave a
/// directory in the user's real Library.
#[test]
fn the_log_directory_is_named_for_the_app() {
    let Some(path) = log_dir()
    else {
        // No home directory - a sandboxed build server, not a failure of
        // this function.
        return;
    };

    let last = path.file_name()
                   .and_then(|name| name.to_str())
                   .expect("the path ends in a name");

    // Case-insensitively, because the platforms disagree on the spelling and
    // both are right: macOS gets `Library/Logs/Knot`, while on Linux
    // `ProjectDirs` follows XDG and lowercases the final component to
    // `~/.local/state/knot`. What matters is that the directory is this
    // app's own rather than a shared one.
    assert!(last.eq_ignore_ascii_case(APP_NAME),
            "the directory is the app's own, not a shared one: {}",
            path.display());
    assert!(path.is_absolute(), "{}", path.display());
}

#[cfg(target_os = "macos")]
#[test]
fn macos_uses_the_platform_log_location() {
    let Some(path) = log_dir()
    else {
        return;
    };

    assert!(path.to_string_lossy().contains("Library/Logs"),
            "macOS keeps logs where Console.app reads them, not under Application Support: {}",
            path.display());
    assert!(!path.to_string_lossy().contains("Caches"),
            "a cache is what the system deletes first: {}",
            path.display());
}
