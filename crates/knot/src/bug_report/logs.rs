//! The logs a report can attach: the application log and the MCP server's.
//!
//! GitHub has no API for attaching a file to an issue, so "attach" means the
//! tail of the log, fenced in a collapsed `<details>` block of the issue body.
//! The browser fallback cannot carry that much in a URL, so it names the
//! files to drag in instead.
//!
//! [`read`] blocks on the file system, so it only runs off the main thread;
//! cutting the tail and laying the block out are pure, and tested without a
//! file.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use crate::consts::{BUG_REPORT_LOG_TAIL_BYTES, BUG_REPORT_LOG_TAIL_LINES};

/// A log a report can carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LogKind {
    /// Knot's own stderr (`app_log`).
    App,
    /// The MCP server's diagnostics log.
    Mcp,
}

impl LogKind {
    pub(crate) const ALL: [Self; 2] = [Self::App, Self::Mcp];

    /// The checkbox's label, and the attachment's heading.
    pub(crate) fn label(self) -> String {
        knot_core::l10n::t(match self {
                               Self::App => "bug_report.logs.app",
                               Self::Mcp => "bug_report.logs.mcp",
                           })
    }

    /// Where the log is written, when there is a log directory at all.
    pub(crate) fn path(self) -> Option<PathBuf> {
        match self {
            Self::App => crate::app_log::app_log_path(),
            Self::Mcp => {
                knot_core::log_dir().map(|directory| directory.join(knot_mcp::LOG_FILE_NAME))
            }
        }
    }
}

/// One log as a report carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Attachment {
    pub(crate) kind: LogKind,
    /// Where it was read from, or would have been.
    pub(crate) path: Option<PathBuf>,
    /// Its tail, or `None` when it could not be read - it may simply not have
    /// been written yet, and a report says so rather than failing.
    pub(crate) tail: Option<String>,
}

/// Reads the tail of `kind`'s log. Blocks.
pub(crate) fn read(kind: LogKind) -> Attachment {
    let path = kind.path();
    let tail = path.as_deref()
                   .and_then(|path| read_tail(path).ok())
                   .map(|bytes| tail(&bytes, BUG_REPORT_LOG_TAIL_LINES));
    Attachment { kind, path, tail }
}

/// The last [`BUG_REPORT_LOG_TAIL_BYTES`] of the file, without reading the
/// rest: the MCP log is rotated at a size cap, but that cap is megabytes.
fn read_tail(path: &Path) -> std::io::Result<Vec<u8>> {
    let mut file = std::fs::File::open(path)?;
    let length = file.metadata()?.len();
    let budget = u64::try_from(BUG_REPORT_LOG_TAIL_BYTES).unwrap_or(u64::MAX);
    let start = length.saturating_sub(budget);
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    // Cut mid-line, the first line is a fragment; drop it rather than
    // attach half an entry.
    if start > 0
       && let Some(newline) = bytes.iter().position(|&byte| byte == b'\n')
    {
        bytes.drain(..=newline);
    }
    Ok(bytes)
}

/// The last `lines` lines of `bytes`, as text. Not valid UTF-8 is replaced
/// rather than refused: a log is attached to be read, and a stray byte must
/// not cost the rest of it.
pub(super) fn tail(bytes: &[u8], lines: usize) -> String {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim_end_matches('\n');
    let kept: Vec<&str> = text.rsplit('\n').take(lines).collect();
    kept.into_iter().rev().collect::<Vec<_>>().join("\n")
}

/// The attachments as the issue body carries them: each a collapsed block
/// under its label, or a line saying it could not be read.
///
/// Fenced with four backticks, so a log line holding three cannot end the
/// block early.
pub(super) fn attachments_markdown(attachments: &[Attachment]) -> String {
    attachments.iter()
               .map(|attachment| {
                   let label = attachment.kind.label();
                   match &attachment.tail {
                       Some(tail) => {
                           format!("<details>\n<summary>{label}</summary>\n\n````\n{tail}\n````\n\n</details>\n")
                       }
                       None => format!("{}\n",
                                       knot_core::l10n::t_with("bug_report.logs.unreadable",
                                                               &[("log", &label),
                                                                 ("path",
                                                                  &display_path(attachment))])),
                   }
               })
               .collect::<Vec<_>>()
               .join("\n")
}

/// The browser fallback's stand-in for the attachments: which files to drag
/// into the page, since a URL cannot carry them.
pub(super) fn attachments_by_path(attachments: &[Attachment]) -> String {
    attachments.iter()
               .map(|attachment| {
                   format!("{}\n",
                           knot_core::l10n::t_with("bug_report.logs.attach_by_hand",
                                                   &[("log", &attachment.kind.label()),
                                                     ("path", &display_path(attachment))]))
               })
               .collect::<Vec<_>>()
               .join("\n")
}

fn display_path(attachment: &Attachment) -> String {
    attachment.path
              .as_deref()
              .map(|path| path.display().to_string())
              .unwrap_or_else(|| knot_core::l10n::t("bug_report.logs.no_directory"))
}

#[cfg(test)]
mod tests;
