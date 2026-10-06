//! A shell agent's terminal session, and an ACP agent's panel session.
//!
//! The terminal itself (the grid, the PTY, the input encodings and the view)
//! is the `gpui-terminal` crate. This crate is what Knot adds on top: which
//! shell an agent runs and what it is told first ([`SessionPlan`]), the
//! activity tracker reading the output, and the pull request scan beside it
//! ([`TerminalSession`]); and the ACP connection a panel-mode agent talks over
//! instead ([`AcpSession`]).

mod acp_session;
mod pull_requests;
mod session;
mod terminal_session;

pub use acp_session::{
    AcpSession, AdapterUpdateStatus, ConnectProgress, ConnectStep, SessionTarget,
    check_adapter_update, update_adapter,
};
pub use session::{SessionConfig, SessionPlan};
pub use terminal_session::TerminalSession;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TerminalError {
    #[error(transparent)]
    Terminal(#[from] gpui_terminal::Error),
}

pub type Result<T> = std::result::Result<T, TerminalError>;
