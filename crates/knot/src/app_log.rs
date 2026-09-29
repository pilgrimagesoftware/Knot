//! The application log: stderr, kept in a file when nobody is reading it.
//!
//! A packaged `Knot.app` is launched with stderr pointed at `/dev/null`, so
//! every `eprintln!` in the workspace - and the message of any panic - was
//! lost, and a bug report had no application log to attach (#515). When
//! stderr is not a terminal it is pointed at [`APP_LOG_FILE_NAME`] in the
//! platform log directory instead. Under `cargo run` it stays on the
//! terminal, where it is already read.
//!
//! One file per run: the previous run's is kept as
//! [`APP_LOG_PREVIOUS_FILE_NAME`], and anything older is replaced. That bounds
//! the log without a rotating writer, which a raw file descriptor could not
//! have anyway.

use std::io::IsTerminal;
use std::path::Path;
use std::path::PathBuf;

use crate::consts::APP_LOG_FILE_NAME;
use crate::consts::APP_LOG_PREVIOUS_FILE_NAME;

/// The application log's path, whether or not this run is writing it.
pub(crate) fn app_log_path() -> Option<PathBuf> {
    knot_core::log_dir().map(|directory| directory.join(APP_LOG_FILE_NAME))
}

/// Points stderr at the application log, unless it is a terminal.
///
/// Called first thing in `run`, before anything has written to stderr. A
/// failure leaves stderr where it was: losing the log must not stop the app.
pub(crate) fn redirect_stderr_to_log() {
    if std::io::stderr().is_terminal() {
        return;
    }
    let Some(directory) = knot_core::log_dir()
    else {
        return;
    };
    if let Err(error) = open_and_redirect(&directory) {
        eprintln!("app log: stderr stays where it was: {error}");
    }
}

fn open_and_redirect(directory: &Path) -> std::io::Result<()> {
    let file = open_fresh_log(directory)?;
    redirect_stderr(&file)
}

/// Keeps the last run's log as the previous one and opens an empty current
/// one.
pub(crate) fn open_fresh_log(directory: &Path) -> std::io::Result<std::fs::File> {
    std::fs::create_dir_all(directory)?;
    let current = directory.join(APP_LOG_FILE_NAME);
    if current.exists() {
        std::fs::rename(&current, directory.join(APP_LOG_PREVIOUS_FILE_NAME))?;
    }
    std::fs::File::create(current)
}

#[cfg(unix)]
fn redirect_stderr(file: &std::fs::File) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;

    // SAFETY: both descriptors are open for the duration of the call - `file`
    // is borrowed, and fd 2 always exists in a process - and `dup2` touches
    // nothing but the descriptor table. Rust's stderr is unbuffered and holds
    // only the number 2, so it follows the new target without being told.
    // `file` may be dropped afterwards: fd 2 is an independent duplicate.
    let result = unsafe { libc::dup2(file.as_raw_fd(), libc::STDERR_FILENO) };
    if result == -1 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(unix))]
fn redirect_stderr(_: &std::fs::File) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests;
