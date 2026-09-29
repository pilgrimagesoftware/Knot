use super::cache_dir;

/// Asserts the shape of the path, never creating it: a test must not leave a
/// directory in the user's real Library.
#[test]
fn the_cache_directory_is_absolute_and_a_cache() {
    let Some(path) = cache_dir()
    else {
        // No home directory - a sandboxed build server, not a failure here.
        return;
    };
    assert!(path.is_absolute(), "{}", path.display());
    if cfg!(target_os = "macos") {
        assert!(path.to_string_lossy().contains("Library/Caches"),
                "{}",
                path.display());
    }
}
