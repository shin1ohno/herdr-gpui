use super::*;

#[test]
fn resolved_addresses_preserve_source_target_and_identity() {
    let address = IpAddr::V4(std::net::Ipv4Addr::new(192, 0, 2, 20));
    let mut peer = Peer {
        name: "Office Mac._ssh._tcp.local.".into(),
        host: Some("studio.local.".into()),
        port: 2222,
        ..Peer::default()
    };
    assert!(peer.candidate(&HashSet::new()).is_none());
    peer.address(address, true, &HashSet::new());
    let found = peer.candidate(&HashSet::new());
    assert!(found.is_some());
    if let Some(found) = found {
        assert_eq!(found.name, "Office Mac");
        assert_eq!(found.target, "ssh://studio.local:2222");
        assert!(found.keys.contains(&address.to_string()));
    }
    peer.address(address, false, &HashSet::new());
    assert!(peer.candidate(&HashSet::new()).is_none());
}

#[test]
fn address_storage_is_bounded_and_duplicate_reports_are_idempotent() {
    let mut peer = Peer::default();
    for octet in 1..=64 {
        let address = IpAddr::V4(std::net::Ipv4Addr::new(192, 0, 2, octet));
        peer.address(address, true, &HashSet::new());
        peer.address(address, true, &HashSet::new());
    }
    assert_eq!(peer.addresses.len(), MAX_ADDRESSES);
    let unique: HashSet<_> = peer.addresses.iter().collect();
    assert_eq!(unique.len(), peer.addresses.len());
}

#[test]
fn a_local_address_beyond_the_storage_cap_still_excludes_this_device() {
    let mut peer = Peer {
        name: "me._ssh._tcp.local.".into(),
        host: Some("me.local.".into()),
        port: 22,
        ..Peer::default()
    };
    for octet in 1..=64 {
        peer.address(
            IpAddr::V4(std::net::Ipv4Addr::new(192, 0, 2, octet)),
            true,
            &HashSet::new(),
        );
    }
    let mine = IpAddr::V4(std::net::Ipv4Addr::new(192, 0, 2, 200));
    peer.address(mine, true, &HashSet::from([mine]));
    assert!(peer.local);
    assert!(peer.candidate(&HashSet::new()).is_none());
}

#[test]
fn cancellation_before_browse_returns_without_creating_a_service() {
    let result = browse(
        Duration::ZERO,
        Duration::ZERO,
        |_| false,
        |_| panic!("closed form"),
        &HashSet::new(),
    );
    assert!(result.is_ok());
}

#[test]
fn native_failures_keep_status_and_source() {
    use std::error::Error as _;
    let denied = Error::status("browsing SSH services", -65570);
    assert!(denied.denied());
    assert!(denied.source().is_some());
    assert!(denied.to_string().contains("-65570"));
    assert!(!Error::status("resolving SSH addresses", -65554).denied());
}
