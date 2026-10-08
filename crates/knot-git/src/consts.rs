use std::time::Duration;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// How often a running `git` is checked for exit.
pub const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// The least time allowed for `git`'s output to finish draining once it has
/// exited, even past the timeout, so a command that exits a moment before its
/// deadline is not reported as timed out for want of a moment to flush.
pub const OUTPUT_DRAIN_FLOOR: Duration = Duration::from_millis(100);

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

/// A folder's working tree root. Absolute already - git resolves it - so
/// unlike [`GIT_COMMON_DIR`] this needs no further canonicalization.
pub const TOPLEVEL: &[&str] = &["rev-parse", "--show-toplevel"];
/// The directory shared by every worktree of one repository. May come back
/// relative to the working directory or already absolute depending on git's
/// version and how the repository was cloned, which is why
/// [`Repository::common_dir`](crate::repository::Repository::common_dir)
/// resolves and canonicalizes it before comparing two worktrees.
pub const GIT_COMMON_DIR: &[&str] = &["rev-parse", "--git-common-dir"];
/// Base argv for a named remote's URL; the remote name is appended.
pub const REMOTE_GET_URL: &[&str] = &["remote", "get-url"];

pub const BRANCH_SHOW_CURRENT: &[&str] = &["branch", "--show-current"];
pub const SHORT_HEAD: &[&str] = &["rev-parse", "--short", "HEAD"];
pub const LOG_UNPUSHED: &[&str] = &["log", "@{u}..", "--oneline"];
pub const AHEAD_BEHIND: &[&str] = &["rev-list", "--left-right", "--count", "@{u}...HEAD"];
