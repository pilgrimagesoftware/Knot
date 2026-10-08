//! Help > Report an Issue: the dialog that collects a report and files it as a
//! GitHub issue (`openspec/specs/bug-reporting`).
//!
//! A modal dialog rather than a window like About: it collects and submits,
//! and a report half-typed underneath another window invites abandonment.
//!
//! `form` decides when Report is available and what the status line says,
//! `diagnostics` gathers and lays out what accompanies every report, `logs`
//! reads the logs a user chooses to attach,
//! `submit` turns a report into a filed issue or a pre-filled browser page,
//! and `dialog` opens, owns and draws the dialog.

mod diagnostics;
mod dialog;
mod form;
mod kind;
mod logs;
mod screenshots;
mod submit;

#[cfg(test)]
pub(crate) use diagnostics::Diagnostics;
pub(crate) use dialog::register_report_issue_action;
#[cfg(test)]
pub(crate) use dialog::{
    BugReport, ReportServices, open_report, register_report_issue_action_with,
};
#[cfg(test)]
pub(crate) use kind::IssueKind;
#[cfg(test)]
pub(crate) use logs::{Attachment, LogKind};
#[cfg(test)]
pub(crate) use submit::{Outcome, Report};
