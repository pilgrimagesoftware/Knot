//! Unit tests for [`super`].

use super::*;

fn addr() -> SocketAddr {
    ([127, 0, 0, 1], 8767).into()
}

fn retrying() -> ServerState {
    ServerState::Retrying { attempt:    3,
                            next_delay: Duration::from_secs(4),
                            error:      "address in use".to_string(), }
}

#[test]
fn running_exposes_only_its_address() {
    let state = ServerState::Running { addr: addr() };
    assert_eq!(state.bound_addr(), Some(addr()));
    assert_eq!(state.attempt(), None);
    assert_eq!(state.next_delay(), None);
    assert_eq!(state.last_error(), None);
}

#[test]
fn retrying_exposes_only_its_attempt_delay_and_error() {
    let state = retrying();
    assert_eq!(state.attempt(), Some(3));
    assert_eq!(state.next_delay(), Some(Duration::from_secs(4)));
    assert_eq!(state.last_error(), Some("address in use"));
    assert_eq!(state.bound_addr(), None);
}

#[test]
fn the_payload_free_states_expose_nothing() {
    for state in [ServerState::Disabled,
                  ServerState::Starting,
                  ServerState::Stopped]
    {
        assert_eq!(state.bound_addr(), None, "{state:?}");
        assert_eq!(state.attempt(), None, "{state:?}");
        assert_eq!(state.next_delay(), None, "{state:?}");
        assert_eq!(state.last_error(), None, "{state:?}");
    }
}

#[test]
fn only_running_is_running() {
    assert!(ServerState::Running { addr: addr() }.is_running());
    for state in [ServerState::Disabled,
                  ServerState::Starting,
                  ServerState::Stopped,
                  retrying()]
    {
        assert!(!state.is_running(), "{state:?}");
    }
}

#[test]
fn only_retrying_is_failing() {
    assert!(retrying().is_failing());
    for state in [ServerState::Disabled,
                  ServerState::Starting,
                  ServerState::Stopped,
                  ServerState::Running { addr: addr() }]
    {
        assert!(!state.is_failing(), "{state:?}");
    }
}

#[test]
fn the_default_state_is_disabled() {
    assert_eq!(ServerState::default(), ServerState::Disabled);
}
