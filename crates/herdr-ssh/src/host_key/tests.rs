#![allow(clippy::unwrap_used)]

use super::*;

/// Generated with `ssh-keygen -t ed25519`; `ssh-keygen -l` printed
/// `SHA256:JOTFRvCkuEl2HbyffolO4A+q4cUOVf0UHhWQLrlaEiU`.
pub(crate) const ED25519: &str =
    "AAAAC3NzaC1lZDI1NTE5AAAAIHooS6ZFNGKNoEEPg0yvrG/ZYGJxNIk98w7H1XHhFMkt";
/// Generated with `ssh-keygen -t ecdsa -b 256`; `ssh-keygen -l` printed
/// `SHA256:6/y82ZDGsnRfjHcVgO4IQbiO7l6Bl9LAQgPtXBu/K1A`.
pub(crate) const ECDSA: &str = "AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTYAAABBBPmTIYhOY6dF78NWUUaRB2ubawF+6DPANdQ1L4iV56Zog7iWgvlgolGisBWNhlDvOZR0ZibgoS+hnspzU2/scVo=";

#[test]
fn fingerprints_match_ssh_keygen() {
    let ed25519 = HostKey::from_known_hosts("ssh-ed25519", ED25519).unwrap();
    assert_eq!(ed25519.algorithm(), "ssh-ed25519");
    assert_eq!(
        ed25519.fingerprint(),
        "SHA256:JOTFRvCkuEl2HbyffolO4A+q4cUOVf0UHhWQLrlaEiU"
    );
    let ecdsa = HostKey::from_known_hosts("ecdsa-sha2-nistp256", ECDSA).unwrap();
    assert_eq!(ecdsa.algorithm(), "ecdsa-sha2-nistp256");
    assert_eq!(
        ecdsa.fingerprint(),
        "SHA256:6/y82ZDGsnRfjHcVgO4IQbiO7l6Bl9LAQgPtXBu/K1A"
    );
}

#[test]
fn known_hosts_fields_round_trip() {
    let key = HostKey::from_known_hosts("ssh-ed25519", ED25519).unwrap();
    assert_eq!(key.known_hosts_fields(), format!("ssh-ed25519 {ED25519}"));
}

#[test]
fn type_field_must_match_the_blob() {
    assert_eq!(HostKey::from_known_hosts("ssh-rsa", ED25519), None);
    assert_eq!(
        HostKey::from_known_hosts("ecdsa-sha2-nistp256", ED25519),
        None
    );
}

#[test]
fn malformed_blobs_are_refused() {
    assert_eq!(
        HostKey::from_known_hosts("ssh-ed25519", "not base64!"),
        None
    );
    assert_eq!(HostKey::from_blob(Vec::new()), None);
    assert_eq!(HostKey::from_blob(vec![0, 0, 0, 9, b'x']), None);
    assert_eq!(HostKey::from_blob(vec![0, 0, 0, 0]), None);
    let mut spaced = Vec::new();
    write_string(&mut spaced, b"ssh ed25519");
    assert_eq!(HostKey::from_blob(spaced), None);
    let mut long = Vec::new();
    write_string(&mut long, "x".repeat(65).as_bytes());
    assert_eq!(HostKey::from_blob(long), None);
}

#[test]
fn equality_is_byte_equality() {
    let a = HostKey::from_known_hosts("ssh-ed25519", ED25519).unwrap();
    let mut blob = a.blob.clone();
    if let Some(last) = blob.last_mut() {
        *last ^= 1;
    }
    let b = HostKey::from_blob(blob).unwrap();
    assert_eq!(a.algorithm(), b.algorithm());
    assert_ne!(a, b);
}

#[test]
fn strings_round_trip() {
    let mut out = Vec::new();
    write_string(&mut out, b"abc");
    write_string(&mut out, b"");
    assert_eq!(out, [0, 0, 0, 3, b'a', b'b', b'c', 0, 0, 0, 0]);
    let (first, rest) = read_string(&out).unwrap();
    assert_eq!(first, b"abc");
    let (second, rest) = read_string(rest).unwrap();
    assert_eq!(second, b"");
    assert!(rest.is_empty());
    assert_eq!(read_string(&[0, 0, 0]), None);
    assert_eq!(read_string(&[0, 0, 0, 2, 1]), None);
}
