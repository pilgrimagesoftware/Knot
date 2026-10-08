//! Which program a shell agent's PTY runs.

use std::ffi::OsString;
use std::path::Path;

use gpui_terminal::PtyCommand;

use crate::consts::{KNOT_AGENT_ENV, KNOT_AGENT_VALUE, USER_SHELL_ARGS};

/// The program an agent terminal starts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum AgentShell {
    /// The user's own shell, interactive (`$SHELL -i`), with their dotfiles.
    /// What every real agent runs.
    #[default]
    User,
    /// A fixed program, run as given and with nothing of the user's sourced.
    ///
    /// For tests: the user's shell drags in whatever their rc files do - a
    /// prompt theme's git daemons, a slow plugin manager, a prompt that waits
    /// on the network - and a test's outcome or runtime should not depend on
    /// the machine's dotfiles.
    Program {
        program: OsString,
        args:    Vec<OsString>,
    },
}

impl AgentShell {
    /// `/bin/sh` with no arguments: present on every Unix, and as an
    /// interactive shell it reads no startup file but `$ENV`.
    pub fn posix_sh() -> Self {
        Self::Program { program: "/bin/sh".into(),
                        args:    Vec::new(), }
    }

    /// The command to start in `folder`, marked as an agent's terminal with
    /// [`KNOT_AGENT_ENV`] whichever program it runs.
    pub(crate) fn command(&self, folder: impl AsRef<Path>) -> PtyCommand {
        let command = match self {
            Self::User => {
                USER_SHELL_ARGS.iter()
                               .fold(PtyCommand::user_shell(), |command, arg| command.arg(arg))
            }
            Self::Program { program, args } => {
                args.iter()
                    .fold(PtyCommand::new(program), |command, arg| command.arg(arg))
            }
        };
        command.cwd(folder.as_ref())
               .env(KNOT_AGENT_ENV, KNOT_AGENT_VALUE)
    }
}
