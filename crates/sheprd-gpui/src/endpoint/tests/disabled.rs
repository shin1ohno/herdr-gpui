use super::*;

#[test]
fn a_disabled_endpoint_never_starts_a_connection() {
    let mut endpoint = Endpoint::new(LOCAL.into(), "Local".into(), ConnectTarget::Local, false);
    endpoint.connect(ConnectOptions::default(), true);
    assert_eq!(endpoint.attempts, 0);
    assert_eq!(endpoint.generation, 0);
    assert!(!endpoint.initial_surface);
    assert!(endpoint.connection.handle.is_none());
    assert_eq!(endpoint.status(), "disabled");
}
