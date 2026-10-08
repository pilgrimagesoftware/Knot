//! The crate's fixed values, in one place.

/// Arguments for the user's own shell in an agent terminal: interactive, so
/// it reads the user's rc files and the agent gets their `PATH`, aliases and
/// tools - the same environment a terminal they opened themselves would have.
pub(crate) const USER_SHELL_ARGS: &[&str] = &["-i"];

/// Set in every agent terminal's environment, to [`KNOT_AGENT_VALUE`], to
/// mark the shell as a Knot agent's rather than one the user opened.
///
/// Dotfiles can test it to skip what only a human at a prompt needs. The
/// case that prompted it: powerlevel10k starts a gitstatusd daemon (and two
/// helper shells) per interactive shell, so a workspace of agents ran one
/// per agent - for a prompt segment no agent reads, while Knot tracks git
/// itself. Knot cannot turn that off from outside: the config p10k generates
/// begins by unsetting every `POWERLEVEL9K_*` variable it inherits. The user
/// opts out after that line instead; `docs/agent-shells.md` has the snippet.
pub(crate) const KNOT_AGENT_ENV: &str = "KNOT_AGENT";

/// [`KNOT_AGENT_ENV`]'s value: any non-empty one would do for `[[ -n ]]`.
pub(crate) const KNOT_AGENT_VALUE: &str = "1";
