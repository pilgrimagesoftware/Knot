//! Stamps the binary with the commit it was built from and the date it was
//! built, so the About window can name the running build and not only its
//! version number (`openspec/specs/about-ui`).
//!
//! Neither value is allowed to fail the build: a binary built from a source
//! tarball has no repository to ask, and `git` may not be installed on the
//! machine doing the building. That case stamps [`UNKNOWN`] rather than an
//! empty string, so the About window can say the commit is unknown instead
//! of showing a blank where an identifier belongs.

use std::path::Path;
use std::process::Command;

/// Stamped in place of the commit when it cannot be determined. The About
/// window matches on this exact value, so the two must stay in step.
const UNKNOWN: &str = "unknown";

fn main() {
    let commit = commit().unwrap_or_else(|| UNKNOWN.to_string());
    println!("cargo:rustc-env=KNOT_BUILD_COMMIT={commit}");
    println!("cargo:rustc-env=KNOT_BUILD_DATE={}", build_date());
    watch_head();
}

/// The short commit hash, suffixed `-dirty` when the working tree has
/// uncommitted changes. `None` when `git` is missing or the directory is not
/// a repository.
fn commit() -> Option<String> {
    let output = Command::new("git").args(["rev-parse", "--short=12", "HEAD"])
                                    .output()
                                    .ok()?;
    if !output.status.success() {
        return None;
    }
    let hash = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if hash.is_empty() {
        return None;
    }
    Some(if is_dirty() {
             format!("{hash}-dirty")
         }
         else {
             hash
         })
}

fn is_dirty() -> bool {
    Command::new("git").args(["status", "--porcelain"])
                       .output()
                       .is_ok_and(|output| output.status.success() && !output.stdout.is_empty())
}

/// Today's UTC date as `YYYY-MM-DD`. Deliberately the build date rather than
/// the commit date, which is unavailable in exactly the no-repository case
/// where a date is still wanted.
fn build_date() -> String {
    let now = time::OffsetDateTime::now_utc();
    format!("{:04}-{:02}-{:02}",
            now.year(),
            u8::from(now.month()),
            now.day())
}

/// Restamps when the checkout moves to another commit or branch, and not on
/// every rebuild. Paths come from `git` itself rather than being assembled
/// from `.git/...`: in a worktree `.git` is a file pointing elsewhere, so the
/// assembled path would watch something that never changes.
///
/// A dirty working tree whose `HEAD` has not moved therefore keeps its
/// previous `-dirty` marker until something else forces a rebuild. That is
/// accepted: the marker is a courtesy for local builds, and released builds
/// come from clean checkouts.
fn watch_head() {
    let Some(head) = git_path("HEAD")
    else {
        return;
    };
    println!("cargo:rerun-if-changed={head}");
    if let Some(reference) = symbolic_ref()
       && let Some(path) = git_path(&reference)
    {
        println!("cargo:rerun-if-changed={path}");
    }
}

/// The absolute path `git` resolves a repository-relative path to, if it
/// exists. A packed ref resolves to a path with no file behind it, which
/// cargo cannot stat - watching it would rebuild every time.
fn git_path(relative: &str) -> Option<String> {
    let output = Command::new("git").args(["rev-parse", "--git-path", relative])
                                    .output()
                                    .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8(output.stdout).ok()?.trim().to_string();
    Path::new(&path).exists().then_some(path)
}

/// The ref `HEAD` points at, or `None` on a detached `HEAD`.
fn symbolic_ref() -> Option<String> {
    let output = Command::new("git").args(["symbolic-ref", "--quiet", "HEAD"])
                                    .output()
                                    .ok()?;
    if !output.status.success() {
        return None;
    }
    let reference = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (!reference.is_empty()).then_some(reference)
}
