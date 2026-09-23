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

    assert!(path.ends_with(APP_NAME),
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
