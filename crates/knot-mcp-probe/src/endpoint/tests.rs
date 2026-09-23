use super::same_endpoint;

const KNOT: &str = "http://127.0.0.1:8767/mcp";

/// The case that makes this a function rather than a string comparison: the
/// user's own registration and Knot's injected one address the same server
/// and rarely spell it the same way.
#[test]
fn loopback_aliases_and_a_trailing_slash_are_the_same_endpoint() {
    assert!(same_endpoint(KNOT, "http://localhost:8767/mcp/"));
    assert!(same_endpoint(KNOT, "http://LOCALHOST:8767/mcp"));
    assert!(same_endpoint(KNOT, "http://[::1]:8767/mcp"));
    assert!(same_endpoint(KNOT, "  http://127.0.0.1:8767/mcp  "));
}

/// A different port is a different server however it is named - two Knots on
/// one machine is an ordinary thing, and merging them would show one row
/// carrying the wrong state.
#[test]
fn a_different_port_is_a_different_server() {
    assert!(!same_endpoint(KNOT, "http://127.0.0.1:9000/mcp"));
    assert!(!same_endpoint(KNOT, "http://localhost:8768/mcp"));
}

#[test]
fn a_different_path_or_host_is_a_different_server() {
    assert!(!same_endpoint(KNOT, "http://127.0.0.1:8767/other"));
    assert!(!same_endpoint(KNOT, "http://127.0.0.1:8767/"));
    assert!(!same_endpoint(KNOT, "http://mcp.example.com:8767/mcp"));
}

/// Knot's server is plain HTTP on the loopback. An `https` entry pointing at
/// it does not work, and merging the two would hide that broken entry behind
/// Knot's own healthy row.
#[test]
fn scheme_is_not_normalized_away() {
    assert!(!same_endpoint(KNOT, "https://127.0.0.1:8767/mcp"));
}

#[test]
fn a_default_port_is_filled_in() {
    assert!(same_endpoint("http://mcp.example.com/mcp",
                          "http://mcp.example.com:80/mcp"));
    assert!(same_endpoint("https://mcp.example.com/mcp",
                          "https://mcp.example.com:443/mcp"));
    assert!(!same_endpoint("http://mcp.example.com/mcp",
                           "http://mcp.example.com:443/mcp"));
}

/// Credentials do not identify a server, and a row must not depend on them.
#[test]
fn userinfo_is_ignored() {
    assert!(same_endpoint("https://user:token@mcp.example.com/mcp",
                          "https://mcp.example.com/mcp"));
}

/// The target comes from another program's output, so "not a URL" has to be
/// an ordinary case. Identical text is still identical; anything else is not
/// worth guessing at.
#[test]
fn unparsable_targets_fall_back_to_exact_text() {
    assert!(same_endpoint("node server.js", "node server.js"));
    assert!(!same_endpoint("node server.js", "node other.js"));
    assert!(!same_endpoint("node server.js", KNOT));
    assert!(!same_endpoint("", KNOT));
}

#[test]
fn a_query_or_fragment_does_not_change_the_host_and_port() {
    assert!(!same_endpoint(KNOT, "http://127.0.0.1:8767/mcp?token=x"),
            "a query is part of the path and may well matter");
    assert!(same_endpoint("http://127.0.0.1:8767/mcp?token=x",
                          "http://localhost:8767/mcp?token=x"));
}
