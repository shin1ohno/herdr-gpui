#![allow(clippy::unwrap_used)]

use russh::client::Handler as _;

use super::*;
use crate::host_key::tests::{ECDSA, ED25519};

const ED25519_KEY: (&str, &str) = ("ssh-ed25519", ED25519);
const ECDSA_KEY: (&str, &str) = ("ecdsa-sha2-nistp256", ECDSA);

fn host_key((algorithm, base64): (&str, &str)) -> HostKey {
    HostKey::from_known_hosts(algorithm, base64).unwrap()
}

fn check(pins: Vec<HostKey>, authenticate: bool, offered: (&str, &str)) -> (bool, Option<Offered>) {
    let (algorithm, base64) = offered;
    let key: PublicKeyOrCertificate = PublicKey::from_openssh(&format!("{algorithm} {base64}"))
        .unwrap()
        .into();
    let slot = OfferedSlot::default();
    let mut client = Client::new(pins, authenticate, Arc::clone(&slot));
    let accepted = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(client.check_server_key(&key))
        .unwrap();
    (accepted, take(&slot))
}

#[test]
fn pinned_key_is_accepted_only_for_authentication() {
    let pins = vec![host_key(ED25519_KEY)];
    assert_eq!(
        check(pins.clone(), true, ED25519_KEY),
        (true, Some(Offered::Pinned))
    );
    // A scan records the match and still disconnects.
    assert_eq!(
        check(pins, false, ED25519_KEY),
        (false, Some(Offered::Pinned))
    );
}

#[test]
fn unpinned_host_offers_a_new_key() {
    assert_eq!(
        check(Vec::new(), true, ED25519_KEY),
        (false, Some(Offered::New(host_key(ED25519_KEY))))
    );
}

#[test]
fn key_of_another_type_is_changed_not_new() {
    // B1: ed25519 is pinned and the host offers only ECDSA.
    assert_eq!(
        check(vec![host_key(ED25519_KEY)], true, ECDSA_KEY),
        (false, Some(Offered::Changed(host_key(ECDSA_KEY))))
    );
}

#[test]
fn any_pin_of_several_matches() {
    let pins = vec![host_key(ED25519_KEY), host_key(ECDSA_KEY)];
    assert_eq!(check(pins, true, ECDSA_KEY), (true, Some(Offered::Pinned)));
}
