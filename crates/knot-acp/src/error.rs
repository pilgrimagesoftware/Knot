use thiserror::Error;

pub type Result<T, E = AcpError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum AcpError {
    #[error("failed to spawn acp adapter: {0}")]
    Spawn(#[from] std::io::Error),

    #[error("acp protocol version {agent} unsupported (client supports {client})")]
    UnsupportedProtocolVersion { client: u32, agent: u32 },

    #[error("agent does not support session/load (resume)")]
    ResumeNotSupported,

    #[error("acp error response (code {code}): {message}")]
    Rpc { code: i64, message: String },

    #[error("acp session ended: {0}")]
    SessionEnded(SessionEndCause),

    #[error("acp connection closed before a response was received")]
    ConnectionClosed,

    /// Connecting or opening a session took too long - per design.md's
    /// "Panel mode fails closed to Terminal mode with a visible error"
    /// requirement, a hung adapter (e.g. blocked on an interactive prompt
    /// it can't show over stdio) must surface an error, not hang forever.
    #[error("timed out waiting for the acp adapter to respond")]
    Timeout,

    /// Auto-installing a missing adapter (design.md decision 6) failed -
    /// either the install command itself couldn't spawn, or it exited
    /// non-zero.
    #[error("failed to install acp adapter: {0}")]
    InstallFailed(String),
}

/// Why an ACP session stopped taking requests, per the `acp-client` spec's
/// subprocess exit / broken pipe / JSON-RPC error handling requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEndCause {
    ProcessExited { code: Option<i32> },
    BrokenPipe,
    RpcError { code: i64, message: String },
}

impl std::fmt::Display for SessionEndCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionEndCause::ProcessExited { code } => match code {
                Some(code) => write!(f, "subprocess exited with code {code}"),
                None => write!(f, "subprocess exited (no exit code)"),
            },
            SessionEndCause::BrokenPipe => write!(f, "broken stdio pipe"),
            SessionEndCause::RpcError { code, message } => {
                write!(f, "rpc error {code}: {message}")
            }
        }
    }
}
