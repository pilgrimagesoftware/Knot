//! What accompanies every report: which binary is running, on what, and
//! whether the forge can be reached.
//!
//! [`Diagnostics::collect`] blocks - it runs `sw_vers`, `uname` and
//! `gh auth status` - so it is only ever called off the main thread.
//! Laying the block out is pure, so it is tested without any of them.

use std::process::{Command, Stdio};

use knot_forge::ForgeAvailability;

use crate::about_window::build_info;

/// The facts the diagnostics block is drawn from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Diagnostics {
    /// Name, version and build, exactly as the About window copies them.
    pub(crate) app:   String,
    pub(crate) os:    String,
    pub(crate) arch:  String,
    pub(crate) forge: ForgeAvailability,
}

impl Diagnostics {
    /// Reads every fact from the running process and the host. Blocks.
    pub(crate) fn collect() -> Self {
        Self { app:   build_info::build_details(),
               os:    os_line(std::env::consts::OS,
                              product_version().as_deref(),
                              kernel_release().as_deref()),
               arch:  std::env::consts::ARCH.to_owned(),
               forge: knot_forge::probe(), }
    }

    /// The block as the dialog shows it and the issue body carries it: one
    /// labelled fact per line. Labels are localized; values are not.
    pub(crate) fn text(&self) -> String {
        [("bug_report.diagnostics.app", self.app.clone()),
         ("bug_report.diagnostics.os", self.os.clone()),
         ("bug_report.diagnostics.arch", self.arch.clone()),
         ("bug_report.diagnostics.forge", forge_label(&self.forge))]
            .into_iter()
            .map(|(key, value)| format!("{}: {value}", knot_core::l10n::t(key)))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// The forge's readiness in words. The three standing states are told apart
/// because each has a different next step for the user.
pub(super) fn forge_label(forge: &ForgeAvailability) -> String {
    match forge {
        ForgeAvailability::Missing => knot_core::l10n::t("bug_report.forge.missing"),
        ForgeAvailability::Unauthenticated => {
            knot_core::l10n::t("bug_report.forge.unauthenticated")
        }
        ForgeAvailability::Ready => knot_core::l10n::t("bug_report.forge.ready"),
        ForgeAvailability::Failed(reason) => {
            knot_core::l10n::t_with("bug_report.forge.failed", &[("reason", reason)])
        }
    }
}

/// `macOS 26.0 (Darwin 25.6.0)`. The product version leads because it is
/// what a user and a maintainer both call the OS; the kernel release follows
/// because it is what is left when the product version cannot be read, and
/// the line must still name something.
pub(super) fn os_line(os: &str, product: Option<&str>, kernel: Option<&str>) -> String {
    let name = match os {
        "macos" => "macOS",
        other => other,
    };
    match (product, kernel) {
        (Some(product), Some(kernel)) => format!("{name} {product} ({kernel})"),
        (Some(product), None) => format!("{name} {product}"),
        (None, Some(kernel)) => format!("{name} ({kernel})"),
        (None, None) => name.to_owned(),
    }
}

/// `sw_vers -productVersion`, on macOS only.
fn product_version() -> Option<String> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    read_command("/usr/bin/sw_vers", &["-productVersion"])
}

/// `uname -sr`: the kernel's name and release, e.g. `Darwin 25.6.0`.
fn kernel_release() -> Option<String> {
    read_command("/usr/bin/uname", &["-sr"])
}

/// A command's trimmed stdout, or `None` when it could not run, failed, or
/// said nothing. Absolute paths: a Finder-launched app's `PATH` is not to be
/// relied on, and both tools live in `/usr/bin` on every host that has them.
fn read_command(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args)
                                      .stdin(Stdio::null())
                                      .stderr(Stdio::null())
                                      .output()
                                      .ok()?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (output.status.success() && !text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(forge: ForgeAvailability) -> Diagnostics {
        Diagnostics { app: build_info::build_details(),
                      os: os_line("macos", Some("26.0"), Some("Darwin 25.6.0")),
                      arch: "aarch64".to_owned(),
                      forge }
    }

    #[test]
    fn the_block_is_one_labelled_fact_per_line() {
        let text = sample(ForgeAvailability::Ready).text();
        let lines: Vec<&str> = text.lines().collect();

        assert_eq!(lines.len(), 4, "{text}");
        for (line, key) in lines.iter().zip(["bug_report.diagnostics.app",
                                             "bug_report.diagnostics.os",
                                             "bug_report.diagnostics.arch",
                                             "bug_report.diagnostics.forge"])
        {
            assert!(line.starts_with(&format!("{}: ", knot_core::l10n::t(key))),
                    "{line} is not labelled by {key}");
        }
    }

    #[test]
    fn the_app_line_is_what_the_about_window_copies() {
        let text = sample(ForgeAvailability::Ready).text();
        let first = text.lines().next().unwrap();

        assert!(first.ends_with(&format!(": {}", build_info::build_details())),
                "{first}");
    }

    #[test]
    fn the_os_line_names_product_and_kernel() {
        assert_eq!(os_line("macos", Some("26.0"), Some("Darwin 25.6.0")),
                   "macOS 26.0 (Darwin 25.6.0)");
    }

    #[test]
    fn the_os_line_falls_back_to_the_kernel() {
        assert_eq!(os_line("macos", None, Some("Darwin 25.6.0")),
                   "macOS (Darwin 25.6.0)");
        assert_eq!(os_line("macos", None, None), "macOS");
        assert_eq!(os_line("linux", None, Some("Linux 6.8.0")),
                   "linux (Linux 6.8.0)");
    }

    #[test]
    fn the_forge_states_are_told_apart() {
        let labels: Vec<String> =
            [ForgeAvailability::Missing,
             ForgeAvailability::Unauthenticated,
             ForgeAvailability::Ready,
             ForgeAvailability::Failed("no such host".to_owned())].iter()
                                                                  .map(forge_label)
                                                                  .collect();

        for (index, label) in labels.iter().enumerate() {
            assert!(!label.starts_with("bug_report."),
                    "{label} rendered its key");
            assert!(labels.iter().skip(index + 1).all(|other| other != label),
                    "{label} is shared by two states");
        }
        assert!(labels[3].contains("no such host"), "{}", labels[3]);
    }

    #[test]
    fn a_command_that_cannot_run_reads_as_none() {
        assert_eq!(read_command("/nonexistent/knot-test-binary", &[]), None);
    }
}
