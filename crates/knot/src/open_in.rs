//! The "Open In…" submenu: the applications an agent's folder can be
//! opened in, and how each one is launched.
//!
//! Ports `Skwad/Models/OpenWithProvider.swift`. Everything goes through
//! `/usr/bin/open`, and a launch that fails - the application is not
//! installed - is swallowed rather than surfaced, per `agent-list-ui`'s
//! "fail quietly" requirement: the user cannot act on "VS Code is not
//! installed" from inside a context menu, and a dialog for it would be
//! worse than nothing happening.

#[cfg(target_os = "macos")]
use std::process::Command;

/// One application in the submenu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OpenInApp {
    pub(crate) id: &'static str,
    pub(crate) label: &'static str,
}

/// A submenu entry: an application, or the divider between the editors and
/// the file/terminal pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenInEntry {
    App(OpenInApp),
    Separator,
}

const VS_CODE: OpenInApp = OpenInApp {
    id: "vscode",
    label: "VS Code",
};
const ZED: OpenInApp = OpenInApp {
    id: "zed",
    label: "Zed",
};
const XCODE: OpenInApp = OpenInApp {
    id: "xcode",
    label: "Xcode",
};
const FINDER: OpenInApp = OpenInApp {
    id: "finder",
    label: "Finder",
};
const TERMINAL: OpenInApp = OpenInApp {
    id: "terminal",
    label: "Terminal",
};

/// The submenu's contents: the editors, a divider, then the two system
/// applications. The reference's order, with Zed added beside the other
/// cross-platform editor rather than after the Apple one - Zed is an
/// addition of this port's own, not something `OpenWithProvider` has.
pub(crate) fn open_in_entries() -> Vec<OpenInEntry> {
    vec![
        OpenInEntry::App(VS_CODE),
        OpenInEntry::App(ZED),
        OpenInEntry::App(XCODE),
        OpenInEntry::Separator,
        OpenInEntry::App(FINDER),
        OpenInEntry::App(TERMINAL),
    ]
}

/// The `open` arguments for `app`, ahead of the folder itself. `None` for
/// an unknown id.
///
/// Split out from the launch so the mapping is testable without running
/// anything: `-b` takes a bundle identifier, `-a` an application name, and
/// getting the two the wrong way round fails silently at the point of use.
fn open_arguments(app_id: &str) -> Option<Vec<&'static str>> {
    match app_id {
        "vscode" => Some(vec!["-b", "com.microsoft.VSCode"]),
        "zed" => Some(vec!["-b", "dev.zed.Zed"]),
        "xcode" => Some(vec!["-a", "Xcode"]),
        // Plain `open <folder>` hands a directory to Finder.
        "finder" => Some(Vec::new()),
        "terminal" => Some(vec!["-b", GHOSTTY_BUNDLE_ID]),
        _ => None,
    }
}

const GHOSTTY_BUNDLE_ID: &str = "com.mitchellh.ghostty";

/// Opens `folder` in the application with this id, quietly doing nothing
/// if the id is unknown, the platform is not macOS, or the application is
/// not installed.
///
/// "Terminal" prefers Ghostty and falls back to Terminal.app, matching the
/// reference - which checks for Ghostty first because that is the terminal
/// Knot itself embeds.
pub(crate) fn open_folder(app_id: &str, folder: &str) {
    let Some(arguments) = open_arguments(app_id) else {
        return;
    };
    let opened = run_open(&arguments, folder);
    if !opened && app_id == "terminal" {
        run_open(&["-a", "Terminal"], folder);
    }
}

#[cfg(target_os = "macos")]
fn run_open(arguments: &[&str], folder: &str) -> bool {
    Command::new("/usr/bin/open")
        .args(arguments)
        .arg(folder)
        .status()
        .is_ok_and(|status| status.success())
}

/// Nothing to open elsewhere: Knot ships on macOS, and the workspace only
/// builds on Linux for CI.
#[cfg(not(target_os = "macos"))]
fn run_open(_arguments: &[&str], _folder: &str) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_submenu_matches_the_swift_reference_order() {
        let labels = open_in_entries()
            .into_iter()
            .map(|entry| match entry {
                OpenInEntry::App(app) => app.label,
                OpenInEntry::Separator => "-",
            })
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            vec!["VS Code", "Zed", "Xcode", "-", "Finder", "Terminal"]
        );
    }

    /// `-b` takes a bundle id and `-a` an application name; swapping them
    /// is a silent no-op at the point of use, which is exactly the failure
    /// this mapping exists to keep out of the menu handlers.
    #[test]
    fn each_application_maps_to_the_right_open_flag() {
        assert_eq!(
            open_arguments("vscode"),
            Some(vec!["-b", "com.microsoft.VSCode"])
        );
        // Zed is launched by bundle id like VS Code, not by name like
        // Xcode. The id was read from the installed application's
        // Info.plist, not recalled.
        assert_eq!(open_arguments("zed"), Some(vec!["-b", "dev.zed.Zed"]));
        assert_eq!(open_arguments("xcode"), Some(vec!["-a", "Xcode"]));
        assert_eq!(open_arguments("finder"), Some(Vec::new()));
        assert_eq!(
            open_arguments("terminal"),
            Some(vec!["-b", "com.mitchellh.ghostty"])
        );
        assert_eq!(open_arguments("nothing-by-that-name"), None);
    }
}
