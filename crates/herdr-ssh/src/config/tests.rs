#![allow(clippy::unwrap_used)]

use super::*;
use crate::host_key::tests::{ECDSA, ED25519};

fn names(algorithms: &[Algorithm]) -> Vec<&str> {
    algorithms.iter().map(Algorithm::as_str).collect()
}

fn ed25519() -> HostKey {
    HostKey::from_known_hosts("ssh-ed25519", ED25519).unwrap()
}

fn ecdsa() -> HostKey {
    HostKey::from_known_hosts("ecdsa-sha2-nistp256", ECDSA).unwrap()
}

#[test]
fn allow_list_has_no_sha1_rsa_compression_or_cbc() {
    let config = config(HOST_KEYS.to_vec());
    let preferred = &config.preferred;
    let kex: Vec<&str> = preferred.kex.iter().map(AsRef::as_ref).collect();
    assert_eq!(
        kex,
        [
            "mlkem768x25519-sha256",
            "curve25519-sha256",
            "curve25519-sha256@libssh.org",
            "ext-info-c",
            "kex-strict-c-v00@openssh.com",
        ]
    );
    assert_eq!(
        names(&preferred.key),
        [
            "ssh-ed25519",
            "ecdsa-sha2-nistp256",
            "ecdsa-sha2-nistp384",
            "ecdsa-sha2-nistp521",
            "rsa-sha2-512",
            "rsa-sha2-256",
        ]
    );
    assert!(preferred.host_key_certificates.is_empty());
    let ciphers: Vec<&str> = preferred.cipher.iter().map(AsRef::as_ref).collect();
    assert_eq!(
        ciphers,
        ["chacha20-poly1305@openssh.com", "aes256-gcm@openssh.com"]
    );
    let macs: Vec<&str> = preferred.mac.iter().map(AsRef::as_ref).collect();
    assert_eq!(
        macs,
        [
            "hmac-sha2-512-etm@openssh.com",
            "hmac-sha2-256-etm@openssh.com"
        ]
    );
    let compression: Vec<&str> = preferred.compression.iter().map(AsRef::as_ref).collect();
    assert_eq!(compression, ["none"]);
}

#[test]
fn policy_timeouts() {
    let config = config(Vec::new());
    assert_eq!(config.keepalive_interval, Some(Duration::from_secs(15)));
    assert_eq!(config.keepalive_max, 4);
    assert_eq!(config.inactivity_timeout, None);
    assert!(config.nodelay);
    assert!(!config.anonymous);
    assert_eq!(CONNECT_DEADLINE, Duration::from_secs(10));
}

#[test]
fn pinned_host_keys_are_only_the_pinned_types() {
    assert_eq!(names(&pinned_host_keys(&[ed25519()])), ["ssh-ed25519"]);
    assert_eq!(
        names(&pinned_host_keys(&[ecdsa(), ed25519()])),
        ["ssh-ed25519", "ecdsa-sha2-nistp256"]
    );
    assert!(pinned_host_keys(&[]).is_empty());
}

#[test]
fn scan_offers_pinned_types_first() {
    assert_eq!(
        names(&scan_host_keys(&[ecdsa()])),
        [
            "ecdsa-sha2-nistp256",
            "ssh-ed25519",
            "ecdsa-sha2-nistp384",
            "ecdsa-sha2-nistp521",
            "rsa-sha2-512",
            "rsa-sha2-256",
        ]
    );
    assert_eq!(names(&scan_host_keys(&[])), names(HOST_KEYS));
}

#[test]
fn rsa_pins_negotiate_sha2_signatures_only() {
    let mut blob = Vec::new();
    crate::host_key::write_string(&mut blob, b"ssh-rsa");
    let key = HostKey::from_blob(blob).unwrap();
    assert_eq!(
        names(&pinned_host_keys(&[key])),
        ["rsa-sha2-512", "rsa-sha2-256"]
    );
}
