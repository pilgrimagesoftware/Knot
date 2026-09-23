//! Deciding whether a server the agent configured *is* Knot's own.
//!
//! A user who ran the settings window's install command has Knot's MCP
//! server in their agent's configuration as well as receiving it through the
//! session Knot opens, so the probe reports it alongside the copy Knot
//! injects. Two rows for one server is wrong, and so is picking by name:
//! `MCP_SERVER_NAME` is only what *Knot* registers under, and the user chose
//! their own when they ran the command by hand.
//!
//! So identity is the endpoint. `localhost` and `127.0.0.1` are the same
//! host, a trailing slash is not a difference, and a different port is a
//! different server however it is named.

/// Hostnames that all mean the loopback interface.
const LOOPBACK_ALIASES: &[&str] = &["localhost", "127.0.0.1", "::1", "[::1]", "0.0.0.0"];

/// Whether two URLs address the same MCP endpoint.
///
/// The scheme must match. `http` and `https` to the same host and port are
/// not treated as one server: Knot's own is plain `http` on the loopback, and
/// an `https` entry pointing at it would not work anyway, so merging them
/// would hide a broken configuration behind Knot's healthy row.
#[must_use]
pub fn same_endpoint(left: &str, right: &str) -> bool {
    match (Endpoint::parse(left), Endpoint::parse(right)) {
        (Some(left), Some(right)) => left == right,
        // Neither is a URL we can read, so fall back to exact text. Two
        // unparsable strings that are byte-identical are still the same
        // thing; two that are not are not worth guessing about.
        _ => left == right,
    }
}

#[derive(Debug, PartialEq, Eq)]
struct Endpoint {
    scheme: String,
    host:   String,
    port:   u16,
    path:   String,
}

impl Endpoint {
    fn parse(url: &str) -> Option<Self> {
        let (scheme, rest) = url.trim().split_once("://")?;
        let scheme = scheme.to_lowercase();

        let default_port = match scheme.as_str() {
            "http" => 80,
            "https" => 443,
            _ => return None,
        };

        let (authority, path) = match rest.find(['/', '?', '#']) {
            Some(at) => (&rest[..at], &rest[at..]),
            None => (rest, ""),
        };

        // Userinfo says nothing about which server this is.
        let authority = authority.rsplit_once('@')
                                 .map_or(authority, |(_, host)| host);
        let (host, port) = split_host_port(authority, default_port)?;

        Some(Self { scheme,
                    host: normalize_host(&host),
                    port,
                    path: normalize_path(path) })
    }
}

/// Splits `host:port`, leaving a bracketed IPv6 literal intact.
fn split_host_port(authority: &str, default_port: u16) -> Option<(String, u16)> {
    if authority.is_empty() {
        return None;
    }

    if let Some(end) = authority.rfind(']') {
        // `[::1]:8767` - the colons inside the brackets are the address.
        let host = &authority[..=end];
        let port = match authority[end + 1..].strip_prefix(':') {
            Some(port) => port.parse().ok()?,
            None => default_port,
        };

        return Some((host.to_owned(), port));
    }

    match authority.rsplit_once(':') {
        Some((host, port)) => Some((host.to_owned(), port.parse().ok()?)),
        None => Some((authority.to_owned(), default_port)),
    }
}

fn normalize_host(host: &str) -> String {
    let lowered = host.to_lowercase();

    if LOOPBACK_ALIASES.contains(&lowered.as_str()) {
        return "localhost".to_owned();
    }

    lowered
}

/// A trailing slash is not a difference, and no path is the root.
fn normalize_path(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');

    if trimmed.is_empty() {
        return "/".to_owned();
    }

    trimmed.to_owned()
}

#[cfg(test)]
mod tests;
