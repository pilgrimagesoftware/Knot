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
///
/// An enum rather than an id string with a `_ => None` arm: the set is
/// fixed, and the arguments each one needs are part of knowing which it is
/// (#224). The only place a string still appears is the menu-bar action,
/// which crosses gpui's action boundary - and [`OpenInApp::from_id`] is the
/// one place that can fail to recognize one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenInApp {
    VsCode,
    Zed,
    Xcode,
    Finder,
    Terminal,
}

/// A submenu entry: an application, or the divider between the editors and
/// the file/terminal pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OpenInEntry {
    App(OpenInApp),
    Separator,
}

impl OpenInApp {
    /// The stable id the menu-bar action carries.
    pub(crate) const fn id(self) -> &'static str {
        match self {
            Self::VsCode => "vscode",
            Self::Zed => "zed",
            Self::Xcode => "xcode",
            Self::Finder => "finder",
            Self::Terminal => "terminal",
        }
    }

    /// The application's own name, which is not translated: these are
    /// product names, the same in every locale.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::VsCode => "VS Code",
            Self::Zed => "Zed",
            Self::Xcode => "Xcode",
            Self::Finder => "Finder",
            Self::Terminal => "Terminal",
        }
    }

    /// The application an id names, or `None` for one this build does not
    /// know - which can only reach here from a stale menu-bar action.
    pub(crate) fn from_id(id: &str) -> Option<Self> {
        [Self::VsCode,
         Self::Zed,
         Self::Xcode,
         Self::Finder,
         Self::Terminal].into_iter()
                        .find(|app| app.id() == id)
    }

    /// The `open` arguments for this application, ahead of the folder
    /// itself.
    ///
    /// `-b` takes a bundle identifier, `-a` an application name, and
    /// getting the two the wrong way round fails silently at the point of
    /// use - which is why this is a value to test rather than a line in
    /// the launcher.
    const fn open_arguments(self) -> &'static [&'static str] {
        match self {
            Self::VsCode => &["-b", "com.microsoft.VSCode"],
            Self::Zed => &["-b", "dev.zed.Zed"],
            Self::Xcode => &["-a", "Xcode"],
            // Plain `open <folder>` hands a directory to Finder.
            Self::Finder => &[],
            Self::Terminal => &["-b", GHOSTTY_BUNDLE_ID],
        }
    }
}

/// The submenu's contents: the editors, a divider, then the two system
/// applications. The reference's order, with Zed added beside the other
/// cross-platform editor rather than after the Apple one - Zed is an
/// addition of this port's own, not something `OpenWithProvider` has.
pub(crate) fn open_in_entries() -> Vec<OpenInEntry> {
    vec![OpenInEntry::App(OpenInApp::VsCode),
         OpenInEntry::App(OpenInApp::Zed),
         OpenInEntry::App(OpenInApp::Xcode),
         OpenInEntry::Separator,
         OpenInEntry::App(OpenInApp::Finder),
         OpenInEntry::App(OpenInApp::Terminal),]
}

const GHOSTTY_BUNDLE_ID: &str = "com.mitchellh.ghostty";

/// Opens `folder` in `app`, quietly doing nothing if the platform is not
/// macOS or the application is not installed.
///
/// "Terminal" prefers Ghostty and falls back to Terminal.app, matching the
/// reference - which checks for Ghostty first because that is the terminal
/// Knot itself embeds.
pub(crate) fn open_folder(app: OpenInApp, folder: &str) {
    let opened = run_open(app.open_arguments(), folder);
    if !opened && app == OpenInApp::Terminal {
        run_open(&["-a", "Terminal"], folder);
    }
}

/// Opens `url` in the user's default browser, returning whether it worked.
///
/// The same `/usr/bin/open` path as [`open_folder`], with no `-a`: the point
/// is the browser the user has chosen, not one Knot picked. A pull request
/// opens there rather than in an embedded view, so it arrives already signed
/// in, with the user's extensions and their session.
///
/// The return value is read, unlike `open_folder`'s: a row that could not be
/// opened has to say so rather than looking like it did nothing.
pub(crate) fn open_url(url: &str) -> bool {
    run_open(&[], url)
}

#[cfg(target_os = "macos")]
fn run_open(arguments: &[&str], folder: &str) -> bool {
    Command::new("/usr/bin/open").args(arguments)
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
        assert_eq!(open_in_entries(),
                   vec![OpenInEntry::App(OpenInApp::VsCode),
                        OpenInEntry::App(OpenInApp::Zed),
                        OpenInEntry::App(OpenInApp::Xcode),
                        OpenInEntry::Separator,
                        OpenInEntry::App(OpenInApp::Finder),
                        OpenInEntry::App(OpenInApp::Terminal)]);
    }

    /// The ids the menu-bar action carries survive a round trip, which is
    /// the only place an application is still named by string.
    #[test]
    fn every_application_round_trips_through_its_id() {
        for app in [OpenInApp::VsCode,
                    OpenInApp::Zed,
                    OpenInApp::Xcode,
                    OpenInApp::Finder,
                    OpenInApp::Terminal]
        {
            assert_eq!(OpenInApp::from_id(app.id()), Some(app));
        }
        assert_eq!(OpenInApp::from_id("nothing-by-that-name"), None);
    }

    /// `-b` takes a bundle id and `-a` an application name; swapping them
    /// is a silent no-op at the point of use, which is exactly the failure
    /// this mapping exists to keep out of the menu handlers.
    #[test]
    fn each_application_maps_to_the_right_open_flag() {
        assert_eq!(OpenInApp::VsCode.open_arguments(),
                   ["-b", "com.microsoft.VSCode"]);
        // Zed is launched by bundle id like VS Code, not by name like
        // Xcode. The id was read from the installed application's
        // Info.plist, not recalled.
        assert_eq!(OpenInApp::Zed.open_arguments(), ["-b", "dev.zed.Zed"]);
        assert_eq!(OpenInApp::Xcode.open_arguments(), ["-a", "Xcode"]);
        assert!(OpenInApp::Finder.open_arguments().is_empty());
        assert_eq!(OpenInApp::Terminal.open_arguments(),
                   ["-b", "com.mitchellh.ghostty"]);
    }
}
