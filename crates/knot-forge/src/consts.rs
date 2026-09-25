use std::time::Duration;

/// The binary Knot reads pull request state through.
pub const GH_PROGRAM: &str = "gh";

/// Matches `knot-git`'s default. A forge request goes over the network, so it
/// is the slower of the two, but the bound exists to stop a hung process from
/// holding a refresh open forever rather than to be tight.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// How often the timeout loop checks whether the child has exited.
pub const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// The fields `gh pr view` is asked for. Everything the view renders and
/// nothing else: asking for less would mean a second call, asking for more
/// would mean paying for data no row shows.
///
/// `mergedAt` is the one field here no row renders, and it earns its place
/// the same way: it is what decides when a merged pull request stops being
/// listed. Measuring that from when Knot first saw the URL instead would be
/// measuring the wrong thing - a sighting is not a merge - so the timestamp
/// has to come from the forge.
pub const PULL_REQUEST_FIELDS: &str =
    "number,title,state,isDraft,mergeable,mergeStateStatus,statusCheckRollup,mergedAt";

/// How long to wait before re-asking when GitHub has not yet computed a pull
/// request's mergeability.
///
/// It computes lazily: the first request for an open pull request usually
/// answers `UNKNOWN` and starts the work, and the next one has the answer.
/// One short retry turns the common case into a single fetch, rather than
/// leaving the row uncoloured until the next refresh a minute later.
pub const MERGEABILITY_RETRY_DELAY: Duration = Duration::from_millis(1200);

/// `gh auth status` says this when it found no credentials. `gh` reports the
/// condition on stderr with a non-zero exit, so the text is what separates
/// "not logged in" from every other failure.
pub const UNAUTHENTICATED_MARKERS: &[&str] = &["not logged in",
                                               "no accounts",
                                               "authentication failed",
                                               "gh auth login"];
