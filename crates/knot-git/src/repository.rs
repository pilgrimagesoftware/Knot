use std::fs;
use std::path::{Path, PathBuf};

use crate::consts;
use crate::diff::{FileDiff, parse_diff};
use crate::error::{GitError, Result};
use crate::runner::Runner;
use crate::stats::{DiffStats, parse_numstat, untracked_line_count};
use crate::status::{RepoStatus, parse_status};

/// A git repository addressed by working directory. All operations run through
/// a [`Runner`], so they share its timeout and working directory.
#[derive(Debug, Clone)]
pub struct Repository {
    runner: Runner,
}

impl Repository {
    pub fn open(path: impl Into<PathBuf>) -> Self {
        Self { runner: Runner::new(path), }
    }

    pub fn with_runner(runner: Runner) -> Self {
        Self { runner }
    }

    pub fn runner(&self) -> &Runner {
        &self.runner
    }

    /// Every file in the working tree a picker should offer: tracked
    /// files plus untracked ones that are not ignored.
    ///
    /// Paths are relative to the repository root and NUL-separated on the
    /// wire, so one containing a space, a quote or a newline survives. A
    /// deleted-but-still-tracked file is included - `ls-files` lists the
    /// index, and a caller wanting only what exists on disk should check.
    pub fn list_files(&self) -> Result<Vec<String>> {
        let output = self.runner.run_raw(consts::LS_FILES)?;
        Ok(output.split('\0')
                 .filter(|path| !path.is_empty())
                 .map(str::to_owned)
                 .collect())
    }

    /// Parsed `git status --porcelain=v2 --branch`.
    pub fn status(&self) -> Result<RepoStatus> {
        let output = self.runner.run(consts::STATUS)?;
        Ok(parse_status(&output))
    }

    /// Combined line changes: `git diff --numstat` plus `git diff --staged
    /// --numstat`, then each untracked file as one changed file with its line
    /// count as insertions (binary or unreadable untracked files add a file
    /// with no line delta).
    pub fn diff_stats(&self) -> Result<DiffStats> {
        let unstaged = parse_numstat(&self.runner.run(consts::NUMSTAT)?);
        let staged = parse_numstat(&self.runner.run(consts::NUMSTAT_STAGED)?);

        let mut stats = DiffStats::default();
        for (added, deleted, _) in unstaged.iter().chain(&staged) {
            stats.insertions += added;
            stats.deletions += deleted;
            stats.files_changed += 1;
        }

        for entry in self.status()?.untracked() {
            let lines = untracked_line_count(fs::read(self.runner.cwd().join(&entry.path)));
            stats.insertions += lines.unwrap_or(0);
            stats.files_changed += 1;
        }

        Ok(stats)
    }

    /// One path's diff: `git diff --no-color [--staged] -- <path> [<orig>]`,
    /// parsed by [`parse_diff`].
    ///
    /// `staged` selects index-against-HEAD rather than worktree-against-index.
    /// They are different diffs for a path that was staged and then modified
    /// again, which is why the caller chooses.
    ///
    /// `orig_path` is a rename's or copy's source, from
    /// [`FileEntry::orig_path`]. Git detects a rename by comparing both sides,
    /// so a diff scoped to the destination alone reports a whole new file
    /// instead - passing the source back is what keeps a rename a rename.
    ///
    /// `None` when the path has no change on that side.
    ///
    /// [`parse_diff`]: crate::diff::parse_diff
    /// [`FileEntry::orig_path`]: crate::status::FileEntry::orig_path
    pub fn file_diff(&self, path: &str, orig_path: Option<&str>, staged: bool)
                     -> Result<Option<FileDiff>> {
        let mut argv = consts::DIFF.to_vec();
        if staged {
            argv.push(consts::DIFF_STAGED_FLAG);
        }
        argv.push(consts::PATHSPEC_SEP);
        argv.push(path);
        argv.extend(orig_path);

        Ok(parse_diff(&self.runner.run(&argv)?).into_iter().next())
    }

    /// `git add <paths>`. Empty slice is a no-op (no process spawned).
    pub fn stage(&self, paths: &[&str]) -> Result<()> {
        self.run_scoped(consts::ADD, paths)
    }

    /// `git restore --staged <paths>`. Empty slice is a no-op.
    pub fn unstage(&self, paths: &[&str]) -> Result<()> {
        self.run_scoped(consts::RESTORE_STAGED, paths)
    }

    /// `git restore <paths>`. Empty slice is a no-op.
    pub fn discard(&self, paths: &[&str]) -> Result<()> {
        self.run_scoped(consts::RESTORE, paths)
    }

    /// `git add -A`.
    pub fn stage_all(&self) -> Result<()> {
        self.runner.run(consts::ADD_ALL).map(drop)
    }

    /// `git reset HEAD`.
    pub fn unstage_all(&self) -> Result<()> {
        self.runner.run(consts::UNSTAGE_ALL).map(drop)
    }

    /// `git commit -m <message>`.
    pub fn commit(&self, message: &str) -> Result<()> {
        let argv = [consts::COMMIT, &[message]].concat();
        self.runner.run(&argv).map(drop)
    }

    /// Current branch from `git branch --show-current`. `None` when HEAD is
    /// detached (empty output).
    pub fn current_branch(&self) -> Result<Option<String>> {
        let branch = self.runner.run(consts::BRANCH_SHOW_CURRENT)?;
        Ok((!branch.is_empty()).then_some(branch))
    }

    /// Abbreviated commit id of `HEAD` from `git rev-parse --short HEAD` -
    /// what names a detached HEAD, where [`Self::current_branch`] has
    /// nothing.
    pub fn short_head(&self) -> Result<String> {
        self.runner.run(consts::SHORT_HEAD)
    }

    /// Whether the branch has commits its upstream lacks
    /// (`git log @{u}.. --oneline`). `false` when there is no upstream.
    pub fn has_unpushed(&self) -> Result<bool> {
        match self.runner.run(consts::LOG_UNPUSHED) {
            Ok(output) => Ok(!output.is_empty()),
            Err(GitError::Command { .. }) => Ok(false),
            Err(err) => Err(err),
        }
    }

    /// `(ahead, behind)` relative to the upstream from
    /// `git rev-list --left-right --count @{u}...HEAD`. `(0, 0)` when there is
    /// no upstream.
    pub fn ahead_behind(&self) -> Result<(u32, u32)> {
        let output = match self.runner.run(consts::AHEAD_BEHIND) {
            Ok(output) => output,
            Err(GitError::Command { .. }) => return Ok((0, 0)),
            Err(err) => return Err(err),
        };

        // left-right count prints "<behind>\t<ahead>": left = @{u}, right =
        // HEAD.
        let mut counts = output.split_whitespace();
        let behind = counts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let ahead = counts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        Ok((ahead, behind))
    }

    /// Create a worktree at `destination` on a new branch `branch`, via
    /// `git worktree add -b <branch> <destination>` run from this repository.
    /// The runner's error propagates unchanged, so an existing branch name
    /// surfaces as [`GitError::Command`] and no worktree is created.
    pub fn create_worktree(&self, branch: &str, destination: &Path) -> Result<()> {
        let destination = destination.to_str().ok_or_else(|| {
            GitError::Parse("worktree destination path is not valid UTF-8".to_owned())
        })?;
        let argv = [consts::WORKTREE_ADD, &[branch, destination]].concat();
        self.runner.run(&argv).map(drop)
    }

    fn run_scoped(&self, base: &[&str], paths: &[&str]) -> Result<()> {
        if paths.is_empty() {
            return Ok(());
        }

        let argv = [base, paths].concat();
        self.runner.run(&argv).map(drop)
    }
}

#[cfg(test)]
mod tests {
    use super::Repository;
    use crate::runner::Runner;

    /// A path-scoped op with no paths must not spawn git. `false` always exits
    /// non-zero, so a spawn would surface as an error.
    #[test]
    fn empty_path_scoped_ops_do_not_spawn() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::with_runner(Runner::new(dir.path()).with_program("false"));

        assert!(repo.stage(&[]).is_ok());
        assert!(repo.unstage(&[]).is_ok());
        assert!(repo.discard(&[]).is_ok());
    }
}
