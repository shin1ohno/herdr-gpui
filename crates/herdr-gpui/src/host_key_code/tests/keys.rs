#![allow(clippy::unwrap_used)]

use super::*;

// Public fixtures and expected ssh-keygen output shared with
// herdr-ssh/src/host_key/tests.rs. No private key material is present.
const ED25519: &str = "AAAAC3NzaC1lZDI1NTE5AAAAIHooS6ZFNGKNoEEPg0yvrG/ZYGJxNIk98w7H1XHhFMkt";
const ECDSA: &str = "AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTYAAABBBPmTIYhOY6dF78NWUUaRB2ubawF+6DPANdQ1L4iV56Zog7iWgvlgolGisBWNhlDvOZR0ZibgoS+hnspzU2/scVo=";

#[test]
fn fingerprint_formula_matches_herdr_ssh_and_ssh_keygen() {
    for (algorithm, key, expected) in [
        (
            "ssh-ed25519",
            ED25519,
            "SHA256:JOTFRvCkuEl2HbyffolO4A+q4cUOVf0UHhWQLrlaEiU",
        ),
        (
            "ecdsa-sha2-nistp256",
            ECDSA,
            "SHA256:6/y82ZDGsnRfjHcVgO4IQbiO7l6Bl9LAQgPtXBu/K1A",
        ),
    ] {
        let fingerprint =
            public_key(format!("{algorithm} {key} host comment\n").as_bytes()).unwrap();
        assert_eq!(fingerprint.algorithm(), algorithm);
        assert_eq!(fingerprint.fingerprint(), expected);
    }
}

#[test]
fn key_type_must_agree_with_the_blob_and_only_one_line_is_allowed() {
    for value in [
        format!("ssh-rsa {ED25519}"),
        format!("ecdsa-sha2-nistp384 {ECDSA}"),
        format!("ssh-dss {ED25519}"),
        format!("ssh-ed25519 {ED25519}\nssh-ed25519 {ED25519}"),
        "ssh-ed25519 not-base64!".into(),
        format!(
            "ssh-ed25519 {}",
            STANDARD.encode(
                [0, 0, 0, 11]
                    .into_iter()
                    .chain(*b"ssh-ed25519")
                    .collect::<Vec<_>>()
            )
        ),
        "-----BEGIN OPENSSH PRIVATE KEY-----".into(),
    ] {
        assert!(public_key(value.as_bytes()).is_none());
    }
}

#[test]
fn every_available_family_is_carried_and_missing_files_are_skipped() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join(HOST_FILES[0]),
        format!("ssh-ed25519 {ED25519}\n"),
    )
    .unwrap();
    std::fs::write(
        root.path().join(HOST_FILES[1]),
        format!("ecdsa-sha2-nistp256 {ECDSA}\n"),
    )
    .unwrap();
    let code = read_in(root.path(), Some("computer.local".into())).unwrap();
    assert!(
        code.fingerprints()
            .iter()
            .any(|key| key.algorithm() == "ssh-ed25519")
    );
    assert!(
        code.fingerprints()
            .iter()
            .any(|key| key.algorithm() == "ecdsa-sha2-nistp256")
    );
    assert_eq!(Code::parse(&code.to_url()).unwrap(), code);
    assert!(!code.to_url().contains(ED25519));
    assert!(!code.to_url().contains(ECDSA));
}

#[test]
fn no_sshd_keys_and_oversized_or_invalid_files_have_typed_failures() {
    let root = tempfile::tempdir().unwrap();
    assert!(matches!(read_in(root.path(), None), Err(Error::NoHostKeys)));
    let path = root.path().join(HOST_FILES[0]);
    for bytes in [vec![b'x'; PUBLIC_KEY_BYTES as usize + 1], vec![0xff]] {
        std::fs::write(&path, bytes).unwrap();
        assert!(
            matches!(read_in(root.path(), None), Err(Error::PublicKey { path: found }) if found == path)
        );
    }
}
