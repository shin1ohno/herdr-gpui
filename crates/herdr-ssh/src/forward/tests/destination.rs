use super::*;

#[test]
fn only_loopback_names_are_destinations() {
    for (host, loopback) in [
        ("localhost", Loopback::Localhost),
        ("127.0.0.1", Loopback::Ipv4),
        ("::1", Loopback::Ipv6),
        ("[::1]", Loopback::Ipv6),
    ] {
        let destination = Destination::parse(host, 8000).unwrap();
        assert_eq!(destination.host(), loopback);
        assert_eq!(destination.port(), 8000);
    }
    // Names are not resolved, so nothing the host's resolver or another
    // address on its loopback range could mean is accepted.
    for host in [
        "",
        "LOCALHOST",
        "localhost.",
        "127.0.0.2",
        "0.0.0.0",
        "::",
        "::ffff:127.0.0.1",
        "192.168.1.10",
        "example.com",
        "localhost:22",
        "127.0.0.1 ",
    ] {
        assert!(
            matches!(Destination::parse(host, 8000), Err(Error::NotLoopback)),
            "{host:?} must be refused"
        );
    }
}

#[test]
fn port_zero_is_refused() {
    assert!(matches!(Destination::localhost(0), Err(Error::InvalidPort)));
    assert!(matches!(
        Destination::new(Loopback::Ipv4, 0),
        Err(Error::InvalidPort)
    ));
    assert_eq!(Destination::localhost(65535).unwrap().port(), 65535);
}

#[test]
fn destinations_print_as_host_and_port() {
    assert_eq!(
        Destination::localhost(80).unwrap().to_string(),
        "localhost:80"
    );
    assert_eq!(
        Destination::new(Loopback::Ipv6, 3000).unwrap().to_string(),
        "[::1]:3000"
    );
    assert_eq!(Loopback::default(), Loopback::Localhost);
}

#[test]
fn a_target_without_pins_never_connects() {
    let target = Target::parse("user@127.0.0.1:1").unwrap();
    let key = crate::client_key::tests::software_key(41);
    for pins in [Pins::None, Pins::Keys(Vec::new())] {
        assert!(matches!(
            Forward::connect(&target, &key, &pins),
            Err(Error::Connect(Failure::HostKeyUnknown))
        ));
    }
}
