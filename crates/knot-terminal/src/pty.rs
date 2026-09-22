use std::io::{Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::thread;

use parking_lot::Mutex;
use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};

use crate::{Result, TerminalError, TerminalTransport};

/// Removes environment variables that identify the *host* terminal app
/// (Warp, iTerm2, etc.) launching Knot from the spawned shell's inherited
/// environment. `portable_pty::CommandBuilder::new` inherits the whole
/// process environment by default; left in place, a shell integration
/// script (e.g. Warp's) sourced by the nested shell's rc files can use
/// these to report status/title directly back to the *host* terminal's own
/// session (by session id/IPC, not by writing to this pty) rather than to
/// the pty Knot actually owns - the nested shell believes it's still
/// running directly inside the host terminal, because it inherited that
/// terminal's session identity.
fn strip_host_terminal_env(command: &mut CommandBuilder) {
    const PREFIXES: &[&str] = &["WARP_", "ITERM_", "KONSOLE_", "VTE_"];
    const EXACT: &[&str] = &["TERM_PROGRAM", "TERM_PROGRAM_VERSION", "TERM_SESSION_ID"];
    for (key, _) in std::env::vars() {
        if EXACT.contains(&key.as_str()) || PREFIXES.iter().any(|prefix| key.starts_with(prefix)) {
            command.env_remove(key);
        }
    }
}

pub struct PtyTransport {
    writer: Mutex<Box<dyn Write + Send>>,
    child:  Arc<Mutex<Box<dyn Child + Send + Sync>>>,
    master: Box<dyn MasterPty + Send>,
}

impl PtyTransport {
    pub fn spawn<Output, Exit>(folder: impl AsRef<Path>, shell: impl Into<String>,
                               on_output: Output, on_exit: Exit)
                               -> Result<Self>
        where Output: Fn(&[u8]) + Send + Sync + 'static,
              Exit: Fn(Option<i32>) + Send + Sync + 'static {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize { rows:         24,
                                                cols:         80,
                                                pixel_width:  0,
                                                pixel_height: 0, })
                             .map_err(|error| TerminalError::Transport(error.to_string()))?;
        let mut command = CommandBuilder::new(shell.into());
        command.arg("-i");
        command.cwd(folder.as_ref());
        strip_host_terminal_env(&mut command);
        let child = pair.slave
                        .spawn_command(command)
                        .map_err(|error| TerminalError::Transport(error.to_string()))?;
        let child = Arc::new(Mutex::new(child));
        let mut reader = pair.master
                             .try_clone_reader()
                             .map_err(|error| TerminalError::Transport(error.to_string()))?;
        let child_for_wait = Arc::clone(&child);
        thread::spawn(move || {
            let mut buffer = [0_u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(size) => on_output(&buffer[..size]),
                }
            }
            let code = child_for_wait.lock()
                                     .wait()
                                     .ok()
                                     .map(|status| status.exit_code() as i32);
            on_exit(code);
        });
        let writer = pair.master
                         .take_writer()
                         .map_err(|error| TerminalError::Transport(error.to_string()))?;
        Ok(Self { writer: Mutex::new(writer),
                  child,
                  master: pair.master })
    }
}

impl Drop for PtyTransport {
    fn drop(&mut self) {
        {
            let mut child = self.child.lock();
            let _ = child.kill();
        }
    }
}

impl TerminalTransport for PtyTransport {
    fn send_text(&mut self, text: &str) -> Result<()> {
        self.writer
            .lock()
            .write_all(text.as_bytes())
            .map_err(|error| TerminalError::Transport(error.to_string()))
    }

    fn send_return(&mut self) -> Result<()> {
        self.send_text("\r")
    }

    fn terminate(&mut self) -> Result<()> {
        self.child
            .lock()
            .kill()
            .map_err(|error| TerminalError::Transport(error.to_string()))
    }

    fn resize(&mut self, rows: u16, cols: u16) -> Result<()> {
        self.master
            .resize(PtySize { rows,
                              cols,
                              pixel_width: 0,
                              pixel_height: 0 })
            .map_err(|error| TerminalError::Transport(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::*;

    #[test]
    fn strip_host_terminal_env_removes_host_identity_vars() {
        // SAFETY: no other test in this process reads these keys, and each
        // is restored/removed before the function returns.
        unsafe {
            std::env::set_var("WARP_TEST_SESSION_UUID", "test-session");
            std::env::set_var("TERM_PROGRAM", "WarpTerminal");
            std::env::set_var("PLAIN_TEST_VAR", "kept");
        }

        let mut command = CommandBuilder::new("/bin/sh");
        strip_host_terminal_env(&mut command);

        assert_eq!(command.get_env("WARP_TEST_SESSION_UUID"), None);
        assert_eq!(command.get_env("TERM_PROGRAM"), None);
        assert_eq!(command.get_env("PLAIN_TEST_VAR"),
                   Some(std::ffi::OsStr::new("kept")));

        unsafe {
            std::env::remove_var("WARP_TEST_SESSION_UUID");
            std::env::remove_var("TERM_PROGRAM");
            std::env::remove_var("PLAIN_TEST_VAR");
        }
    }

    #[test]
    fn pty_forwards_output_and_exit_status() {
        let folder = tempfile::tempdir().unwrap();
        let (output_tx, output_rx) = mpsc::channel();
        let (exit_tx, exit_rx) = mpsc::channel();
        let mut transport = PtyTransport::spawn(folder.path(),
                                                "/bin/sh",
                                                move |bytes| {
                                                    let _ = output_tx.send(bytes.to_vec());
                                                },
                                                move |status| {
                                                    let _ = exit_tx.send(status);
                                                }).unwrap();

        transport.send_text("printf ready; exit 3\n").unwrap();
        let status = exit_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let output = output_rx.try_iter().flatten().collect::<Vec<_>>();

        assert_eq!(status, Some(3));
        assert!(String::from_utf8_lossy(&output).contains("ready"));
    }
}
