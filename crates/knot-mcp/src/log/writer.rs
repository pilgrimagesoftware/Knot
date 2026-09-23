//! The one task that owns the log file.
//!
//! Exactly one writer exists per log, which is what makes rotation safe with
//! no locking: there is no window in which a second thread writes into a
//! file that is being rolled. It also means the byte counter here is the
//! only accounting needed - the writer knows the file's size because it is
//! the only thing that has changed it.
//!
//! Nothing in here may fail upward. A log that cannot be written must not
//! fail a request or stop the server, so every error ends at
//! [`Writer::report`] and the task keeps draining its channel either way.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use tokio::sync::mpsc::UnboundedReceiver;

use crate::consts;
use crate::log::entry::Entry;

/// Owns the active file, its size, and the state of the current failure
/// episode.
pub(super) struct Writer {
    /// Where the active log file lives. Rolled files are siblings, this path
    /// with a `.1`, `.2`, ... extension appended.
    path:     PathBuf,
    /// `None` while the file cannot be opened. The writer goes on draining
    /// its channel in that state - the sender side must behave the same
    /// whether logging works or not, so no call site has to know.
    file:     Option<File>,
    /// Bytes in the active file, seeded from its length on open so an
    /// appended-to file rolls at the cap rather than at the cap plus
    /// whatever was already there.
    written:  u64,
    /// Whether a failure has already been reported. One episode is one
    /// report: a log that cannot be written must not print a line of
    /// complaint for every entry it could not write.
    degraded: bool,
    /// The size the active file rolls at, and how many rolled files survive.
    /// Fields rather than constants read in place so a test can drive a
    /// rotation in bytes instead of writing megabytes to prove the same
    /// thing; production seeds both from `consts`.
    cap:      u64,
    retained: usize,
}

impl Writer {
    /// Opens `path` for appending, creating its directory if needed. A
    /// failure here is reported once and leaves a writer that still accepts
    /// and drains entries.
    pub(super) fn open(path: PathBuf) -> Self {
        Self::open_with(path, consts::LOG_ROTATION_SIZE, consts::LOG_RETAINED_FILES)
    }

    fn open_with(path: PathBuf, cap: u64, retained: usize) -> Self {
        let mut writer = Self { path,
                                file: None,
                                written: 0,
                                degraded: false,
                                cap,
                                retained };
        writer.reopen();
        writer
    }

    /// Drains `entries` until every sender is dropped.
    pub(super) async fn run(mut self, mut entries: UnboundedReceiver<Entry>) {
        while let Some(entry) = entries.recv().await {
            self.write(&entry);
        }
    }

    /// Writes one entry, rolling first when it would take the file past the
    /// cap.
    ///
    /// Rolling *before* the write rather than after is what makes the
    /// guarantee that rotation loses nothing and interleaves nothing: the
    /// entry lands wholly in one file or wholly in the next, never split
    /// across the rename.
    fn write(&mut self, entry: &Entry) {
        let mut line = entry.render();
        line.push('\n');
        let size = line.len() as u64;

        // `written > 0` keeps a single entry larger than the whole cap from
        // rolling an empty file aside before writing itself into the next
        // one: it would exceed the cap wherever it went.
        if self.file.is_some() && self.written > 0 && self.written + size > self.cap {
            self.roll();
        }
        let Some(file) = self.file.as_mut()
        else {
            return;
        };
        match file.write_all(line.as_bytes()).and_then(|()| file.flush()) {
            Ok(()) => {
                self.written += size;
                self.recovered();
            }
            Err(error) => {
                let path = self.path.display();
                self.report(format!("knot-mcp: cannot write {path}: {error}"));
            }
        }
    }

    /// Renames the active file aside, shifts the rolled files up, deletes
    /// anything past the retained count, and opens a new active file.
    fn roll(&mut self) {
        // Dropped before the renames: a file being renamed while this
        // process still holds it open is legal on Unix and confusing
        // everywhere else, and the handle is about to be replaced anyway.
        self.file = None;

        // Oldest first, so nothing is overwritten on its way up. The
        // retained count names how many rolled files survive, so `.N` is
        // the last one kept and anything at `.N+1` would be the one this
        // roll pushes past the limit.
        for index in (1..=self.retained).rev() {
            let from = rolled(&self.path, index);
            if index == self.retained {
                let _ = std::fs::remove_file(&from);
            }
            else {
                let _ = std::fs::rename(&from, rolled(&self.path, index + 1));
            }
        }
        let _ = std::fs::rename(&self.path, rolled(&self.path, 1));
        self.reopen();
    }

    /// Opens the active file for appending, creating its directory if it
    /// does not exist, and seeds the byte counter from what is already
    /// there.
    fn reopen(&mut self) {
        if let Some(directory) = self.path.parent()
           && let Err(error) = std::fs::create_dir_all(directory)
        {
            let directory = directory.display();
            self.report(format!("knot-mcp: cannot create {directory}: {error}"));
            self.file = None;
            return;
        }
        match OpenOptions::new().create(true)
                                .append(true)
                                .open(&self.path)
        {
            Ok(file) => {
                self.written = file.metadata().map(|data| data.len()).unwrap_or(0);
                self.file = Some(file);
                self.recovered();
            }
            Err(error) => {
                let path = self.path.display();
                self.report(format!("knot-mcp: cannot open {path}: {error}"));
                self.file = None;
            }
        }
    }

    /// Reports the first failure of an episode and stays silent for the
    /// rest of it.
    fn report(&mut self, message: String) {
        if !self.degraded {
            self.degraded = true;
            eprintln!("{message} - logging to stderr only");
        }
    }

    /// Ends the current failure episode, if one is open. Silent by design:
    /// a log that started working again is not news, and saying so on every
    /// successful write after a failure would be its own flood.
    fn recovered(&mut self) {
        self.degraded = false;
    }
}

/// The path of the `index`-th rolled file beside `path`.
fn rolled(path: &Path, index: usize) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(format!(".{index}"));
    PathBuf::from(name)
}

#[cfg(test)]
mod tests;
