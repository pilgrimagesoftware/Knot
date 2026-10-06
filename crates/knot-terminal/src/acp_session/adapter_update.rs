//! Whether an npm-installed ACP adapter (`claude-agent-acp`, `codex-acp`)
//! has a newer version published than the one on `PATH`, and installing it
//! if so.
//!
//! Separate from the auto-install in `super::run_install`, which only ever
//! runs when the adapter binary is missing (`ErrorKind::NotFound`) - this
//! module is for a user-initiated check/update once an adapter is already
//! working, per issue #598.

use knot_acp::{AcpError, Result as AcpResult};
use knot_agent_launch::{InstallMethod, adapter_path};
use tokio::process::Command;

/// The result of comparing an adapter's locally installed npm package
/// version against the latest one published to the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterUpdateStatus {
    /// `None` when `npm list -g` doesn't know the package at all - the
    /// adapter runs from somewhere other than the declared npm install
    /// (e.g. linked locally during development), so there is nothing to
    /// compare against and [`Self::update_available`] is always `false`.
    pub installed_version: Option<String>,
    pub latest_version:    String,
    pub update_available:  bool,
}

/// Checks `install.package`'s installed and latest npm versions and
/// reports whether a newer one is available. Read-only: never installs
/// anything, so it's safe to run on a timer or a button press without side
/// effects.
pub async fn check_adapter_update(install: InstallMethod) -> AcpResult<AdapterUpdateStatus> {
    let latest_version = npm_view_latest_version(install.package).await?;
    let installed_version = npm_list_installed_version(install.package).await?;
    let update_available =
        installed_version.as_deref()
                         .is_some_and(|installed| {
                             version_cmp(installed, &latest_version) == std::cmp::Ordering::Less
                         });
    Ok(AdapterUpdateStatus { installed_version,
                             latest_version,
                             update_available })
}

/// Installs `install`'s package - the same command the auto-install path
/// runs on a missing binary, exposed here so a caller can re-run it
/// on-demand once [`check_adapter_update`] reports a newer version.
pub async fn update_adapter(install: InstallMethod) -> AcpResult<()> {
    super::run_install(install).await
}

/// `npm view <package> version`'s stdout is the latest published version,
/// alone on one line.
async fn npm_view_latest_version(package: &str) -> AcpResult<String> {
    let output = Command::new("npm")
        .args(["view", package, "version"])
        .env("PATH", adapter_path())
        .output()
        .await
        .map_err(|error| {
            AcpError::UpdateCheckFailed(format!(
                "failed to run `npm view {package} version`: {error}"
            ))
        })?;
    if !output.status.success() {
        return Err(AcpError::UpdateCheckFailed(format!(
            "`npm view {package} version` exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if version.is_empty() {
        return Err(AcpError::UpdateCheckFailed(format!("`npm view {package} version` returned no version")));
    }
    Ok(version)
}

/// `npm list -g <package> --depth=0 --json`'s installed version, or `None`
/// if the package isn't present under `dependencies` - `npm list` exits
/// non-zero in that case but still writes valid JSON, so a non-zero status
/// alone isn't an error; only unparsable output is.
async fn npm_list_installed_version(package: &str) -> AcpResult<Option<String>> {
    let output = Command::new("npm")
        .args(["list", "-g", package, "--depth=0", "--json"])
        .env("PATH", adapter_path())
        .output()
        .await
        .map_err(|error| {
            AcpError::UpdateCheckFailed(format!(
                "failed to run `npm list -g {package} --depth=0 --json`: {error}"
            ))
        })?;
    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        AcpError::UpdateCheckFailed(format!(
            "failed to parse `npm list -g {package}` output: {error}"
        ))
    })?;
    Ok(parsed.get("dependencies")
             .and_then(|dependencies| dependencies.get(package))
             .and_then(|entry| entry.get("version"))
             .and_then(|version| version.as_str())
             .map(str::to_string))
}

/// Compares two version strings component-wise as unsigned integers (e.g.
/// `"1.9.0"` < `"1.10.0"`, where a plain string compare would get this
/// backwards). A component that isn't purely numeric is truncated at its
/// first non-digit character and missing trailing components compare as
/// `0` - good enough to rank two release versions, but not full semver:
/// `"1.2.0-beta.1"`'s `-beta` suffix is dropped from its own component
/// (becoming `0`) while `.1` becomes a fourth component, so it compares as
/// *greater* than `"1.2.0"` rather than semver's "pre-release sorts before
/// its release". `npm view <package> version` resolves the `latest` dist
/// tag, which is never a pre-release for these adapters, so this case
/// doesn't arise in practice.
fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |version: &str| -> Vec<u64> {
        version.split('.')
               .map(|part| {
                   let digits: String = part.chars().take_while(char::is_ascii_digit).collect();
                   digits.parse().unwrap_or(0)
               })
               .collect()
    };
    let (a, b) = (parse(a), parse(b));
    let len = a.len().max(b.len());
    for index in 0..len {
        let ordering = a.get(index)
                        .copied()
                        .unwrap_or(0)
                        .cmp(&b.get(index).copied().unwrap_or(0));
        if ordering != std::cmp::Ordering::Equal {
            return ordering;
        }
    }
    std::cmp::Ordering::Equal
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::*;

    #[test]
    fn equal_versions_compare_equal() {
        assert_eq!(version_cmp("1.2.3", "1.2.3"), Ordering::Equal);
    }

    #[test]
    fn patch_version_difference_is_detected() {
        assert_eq!(version_cmp("1.2.3", "1.2.4"), Ordering::Less);
        assert_eq!(version_cmp("1.2.4", "1.2.3"), Ordering::Greater);
    }

    /// The reason for a numeric compare rather than a string compare: a
    /// lexicographic compare puts "1.9.0" after "1.10.0".
    #[test]
    fn double_digit_minor_version_sorts_numerically_not_lexically() {
        assert_eq!(version_cmp("1.9.0", "1.10.0"), Ordering::Less);
    }

    #[test]
    fn missing_trailing_components_compare_as_zero() {
        assert_eq!(version_cmp("1.2", "1.2.0"), Ordering::Equal);
        assert_eq!(version_cmp("1.2", "1.2.1"), Ordering::Less);
    }

    /// Documents the known limitation named on `version_cmp`'s doc comment
    /// rather than a desired behavior: this is not full semver precedence.
    #[test]
    fn prerelease_suffix_is_not_given_semver_precedence() {
        assert_eq!(version_cmp("1.2.0-beta.1", "1.2.0"), Ordering::Greater);
    }

    #[test]
    fn update_available_is_false_when_versions_match() {
        let status = AdapterUpdateStatus { installed_version: Some("1.2.3".to_string()),
                                           latest_version:    "1.2.3".to_string(),
                                           update_available:  version_cmp("1.2.3", "1.2.3")
                                                              == Ordering::Less, };
        assert!(!status.update_available);
    }

    #[test]
    fn update_available_is_true_when_latest_is_newer() {
        let update_available = version_cmp("1.2.3", "1.3.0") == Ordering::Less;
        assert!(update_available);
    }
}
