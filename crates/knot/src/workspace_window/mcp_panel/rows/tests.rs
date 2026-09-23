use knot_mcp_probe::{Inventory, ServerRow, ServerState, Target};

use super::{SectionRow, compose};

const KNOT_URL: &str = "http://127.0.0.1:8767/mcp";

fn running() -> knot_mcp::ServerState {
    knot_mcp::ServerState::Running { addr: "127.0.0.1:8767".parse().expect("an address"), }
}

fn http(name: &str, url: &str, state: ServerState) -> ServerRow {
    ServerRow::new(name, Target::Http { url: url.to_owned(), }, state)
}

fn agent_names(rows: &[SectionRow]) -> Vec<&str> {
    rows.iter()
        .filter_map(|row| match row {
            SectionRow::Agent(row) => Some(row.name.as_str()),
            SectionRow::Knot(_) => None,
        })
        .collect()
}

fn knot_row(rows: &[SectionRow]) -> &super::KnotRow {
    match rows.first().expect("a first row") {
        SectionRow::Knot(knot) => knot,
        SectionRow::Agent(row) => panic!("expected Knot's row first, got {}", row.name),
    }
}

#[test]
fn knots_row_comes_first_and_the_agents_follow() {
    let inventory =
        Inventory::probed(vec![http("github",
                                    "https://github.example/mcp",
                                    ServerState::Connected),
                               http("sentry", "https://sentry.example/mcp", ServerState::Failed)]);

    let rows = compose(running(), KNOT_URL, Some(&inventory));

    assert_eq!(rows.len(), 3);
    assert_eq!(knot_row(&rows).state, running());
    assert_eq!(agent_names(&rows), ["github", "sentry"]);
}

/// Knot's row is not waiting on anything: its state comes from Knot's own
/// supervisor, so it draws before any probe has completed.
#[test]
fn knots_row_is_present_before_any_probe() {
    let rows = compose(running(), KNOT_URL, None);

    assert_eq!(rows.len(), 1);
    assert_eq!(knot_row(&rows).state, running());
    assert!(!knot_row(&rows).also_configured);
}

/// Present even when off. Omitting it would make "the MCP server is disabled"
/// indistinguishable from "Knot has no MCP server", and the first is a
/// setting the user can change.
#[test]
fn knots_row_is_present_when_disabled_or_stopped() {
    for state in [knot_mcp::ServerState::Disabled,
                  knot_mcp::ServerState::Stopped,
                  knot_mcp::ServerState::Starting]
    {
        let rows = compose(state.clone(), KNOT_URL, Some(&Inventory::probed(vec![])));

        assert_eq!(rows.len(), 1, "{state:?} lost Knot's row");
        assert_eq!(knot_row(&rows).state, state);
    }
}

/// The double-registration case. A user who ran the settings window's install
/// command has Knot's server in their agent's configuration too, so the probe
/// reports it alongside the copy Knot injects.
#[test]
fn knots_server_registered_by_hand_merges_into_one_row() {
    let inventory = Inventory::probed(vec![http("github",
                                                "https://github.example/mcp",
                                                ServerState::Connected),
                                           // Same endpoint, different spelling and a name
                                           // the user chose.
                                           http("my-knot",
                                                "http://localhost:8767/mcp/",
                                                ServerState::Connected)]);

    let rows = compose(running(), KNOT_URL, Some(&inventory));

    assert_eq!(rows.len(),
               2,
               "the hand-registered copy became a second row");
    assert!(knot_row(&rows).also_configured,
            "the user should be told their agent configures it independently");
    assert_eq!(agent_names(&rows), ["github"]);
}

/// The merged row carries Knot's own state, not the probe's. Knot's
/// supervisor is authoritative and instant; the probe is a separate process's
/// health check taken at some earlier moment.
#[test]
fn the_merged_row_keeps_knots_own_state() {
    let inventory = Inventory::probed(vec![http("knot", KNOT_URL, ServerState::Failed)]);

    let rows = compose(running(), KNOT_URL, Some(&inventory));

    assert_eq!(rows.len(), 1);
    assert_eq!(knot_row(&rows).state,
               running(),
               "the probe's stale view overwrote Knot's own");
}

/// Identity is the endpoint. A server named like Knot's but pointing
/// elsewhere is somebody else's server and must stay its own row.
#[test]
fn a_server_named_like_knots_but_pointing_elsewhere_is_not_merged() {
    let inventory =
        Inventory::probed(vec![http("knot", "http://127.0.0.1:9999/mcp", ServerState::Connected)]);

    let rows = compose(running(), KNOT_URL, Some(&inventory));

    assert_eq!(rows.len(), 2);
    assert!(!knot_row(&rows).also_configured);
    assert_eq!(agent_names(&rows), ["knot"]);
}

/// Knot supervises and restarts its own server, and its configuration is the
/// settings window's MCP tab. There is no agent flow to hand over, in any
/// state - including the failed one, where the temptation is greatest.
#[test]
fn knots_row_never_offers_the_delegated_action() {
    for state in [knot_mcp::ServerState::Disabled,
                  knot_mcp::ServerState::Starting,
                  running(),
                  knot_mcp::ServerState::Stopped,
                  knot_mcp::ServerState::Retrying { attempt:    3,
                                                    next_delay: std::time::Duration::from_secs(2),
                                                    error:      "connection refused".to_owned(), }]
    {
        let rows = compose(state.clone(), KNOT_URL, None);

        assert!(!rows[0].offers_action(),
                "{state:?} offered an action on Knot's own row");
    }
}

#[test]
fn an_agent_row_offers_the_action_exactly_when_its_state_does() {
    let inventory =
        Inventory::probed(vec![http("a", "https://a.example/mcp", ServerState::Connected),
                               http("b",
                                    "https://b.example/mcp",
                                    ServerState::NeedsAuthentication),
                               http("c", "https://c.example/mcp", ServerState::Disabled),
                               http("d", "https://d.example/mcp", ServerState::Unknown)]);

    let rows = compose(running(), KNOT_URL, Some(&inventory));

    assert!(!rows[1].offers_action(), "connected");
    assert!(rows[2].offers_action(), "needs authentication");
    assert!(rows[3].offers_action(),
            "disabled is re-enabled through the agent's own flow");
    assert!(!rows[4].offers_action(), "unknown has no known remedy");
}

/// An agent type Knot cannot interrogate still shows Knot's own row - it
/// knows that one regardless.
#[test]
fn an_unprobeable_agent_still_shows_knots_row() {
    let rows = compose(running(), KNOT_URL, Some(&Inventory::Unprobeable));

    assert_eq!(rows.len(), 1);
    assert_eq!(knot_row(&rows).state, running());
}
