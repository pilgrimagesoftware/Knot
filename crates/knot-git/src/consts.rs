use std::time::Duration;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// The binary this crate runs, until the application names another through
/// [`crate::program::configure`].
pub const GIT_PROGRAM: &str = "git";

pub const VERSION: &[&str] = &["--version"];

pub const STATUS: &[&str] = &["status", "--porcelain=v2", "--branch"];

/// Every file the repository tracks, plus the untracked ones git would
/// not ignore. `--exclude-standard` is what honours `.gitignore`, the
/// global excludes file and `.git/info/exclude`, so a caller listing files
/// for a picker does not have to reimplement ignore rules.
///
/// `-z` because a path may contain anything but NUL, and without it git
/// C-quotes the awkward ones - which would have to be unquoted to be used.
pub const LS_FILES: &[&str] = &["ls-files",
                                "--cached",
                                "--others",
                                "--exclude-standard",
                                "-z"];

pub const DIFF: &[&str] = &["diff", "--no-color"];
pub const DIFF_STAGED_FLAG: &str = "--staged";
/// Separates revisions from paths. Not optional on a path-scoped diff: a path
/// that also names a branch is otherwise ambiguous, and git resolves it as the
/// revision.
pub const PATHSPEC_SEP: &str = "--";

pub const NUMSTAT: &[&str] = &["diff", "--numstat"];
pub const NUMSTAT_STAGED: &[&str] = &["diff", "--staged", "--numstat"];

pub const ADD: &[&str] = &["add"];
pub const ADD_ALL: &[&str] = &["add", "-A"];
pub const RESTORE_STAGED: &[&str] = &["restore", "--staged"];
pub const UNSTAGE_ALL: &[&str] = &["reset", "HEAD"];
pub const RESTORE: &[&str] = &["restore"];
pub const COMMIT: &[&str] = &["commit", "-m"];

pub const WORKTREE_ADD: &[&str] = &["worktree", "add", "-b"];

pub const BRANCH_SHOW_CURRENT: &[&str] = &["branch", "--show-current"];
pub const LOG_UNPUSHED: &[&str] = &["log", "@{u}..", "--oneline"];
pub const AHEAD_BEHIND: &[&str] = &["rev-list", "--left-right", "--count", "@{u}...HEAD"];
