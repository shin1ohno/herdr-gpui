#![allow(clippy::unwrap_used)]

use std::os::unix::fs::PermissionsExt as _;
use std::sync::{Arc, Barrier};

use super::*;
use crate::host_key::tests::{ECDSA, ED25519};

fn target(text: &str) -> Target {
    Target::parse(text).unwrap()
}

fn ed25519() -> HostKey {
    HostKey::from_known_hosts("ssh-ed25519", ED25519).unwrap()
}

fn ecdsa() -> HostKey {
    HostKey::from_known_hosts("ecdsa-sha2-nistp256", ECDSA).unwrap()
}

fn file_with(text: &str) -> (tempfile::TempDir, KnownHosts) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("known_hosts");
    fs::write(&path, text).unwrap();
    (dir, KnownHosts::new(path))
}

fn contents(hosts: &KnownHosts) -> String {
    fs::read_to_string(&hosts.path).unwrap()
}

#[test]
fn missing_file_pins_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let hosts = KnownHosts::new(dir.path().join("absent"));
    assert_eq!(hosts.pins(&target("a@mac")).unwrap(), Pins::None);
}

#[test]
fn pins_gather_every_key_type() {
    let (_dir, hosts) = file_with(&format!(
        "mac ssh-ed25519 {ED25519}\nother ssh-ed25519 {ED25519}\nmac ecdsa-sha2-nistp256 {ECDSA}\n"
    ));
    assert_eq!(
        hosts.pins(&target("a@mac")).unwrap(),
        Pins::Keys(vec![ed25519(), ecdsa()])
    );
}

#[test]
fn port_is_part_of_the_name() {
    let (_dir, hosts) = file_with(&format!("[mac]:2222 ssh-ed25519 {ED25519}\n"));
    assert_eq!(hosts.pins(&target("a@mac")).unwrap(), Pins::None);
    assert_eq!(
        hosts.pins(&target("a@mac:2222")).unwrap(),
        Pins::Keys(vec![ed25519()])
    );
}

#[test]
fn names_match_case_insensitively_in_lists() {
    let (_dir, hosts) = file_with(&format!(
        "  MAC.local,10.0.0.2 ssh-ed25519 {ED25519} comment words\r\n"
    ));
    assert_eq!(
        hosts.pins(&target("a@mac.local")).unwrap(),
        Pins::Keys(vec![ed25519()])
    );
    assert_eq!(
        hosts.pins(&target("a@10.0.0.2")).unwrap(),
        Pins::Keys(vec![ed25519()])
    );
}

#[test]
fn negation_excludes_the_line() {
    let (_dir, hosts) = file_with(&format!("mac,!mac ssh-ed25519 {ED25519}\n"));
    assert_eq!(hosts.pins(&target("a@mac")).unwrap(), Pins::None);
}

#[test]
fn comments_markers_hashes_and_wildcards_are_skipped() {
    let (_dir, hosts) = file_with(&format!(
        "# mac ssh-ed25519 {ED25519}\n\
         @cert-authority mac ssh-ed25519 {ED25519}\n\
         @revoked mac ssh-ed25519 {ED25519}\n\
         |1|c2FsdA==|aGFzaA== ssh-ed25519 {ED25519}\n\
         ma* ssh-ed25519 {ED25519}\n\
         \n"
    ));
    assert_eq!(hosts.pins(&target("a@mac")).unwrap(), Pins::None);
}

#[test]
fn malformed_pin_for_the_host_is_an_error() {
    let (_dir, hosts) = file_with("mac ssh-ed25519 !!!\n");
    assert!(matches!(
        hosts.pins(&target("a@mac")),
        Err(Error::KnownHosts {
            operation: "has a malformed entry for this host",
            ..
        })
    ));
    // A mismatched type is as damaged as bad base64.
    let (_dir, hosts) = file_with(&format!("mac ssh-rsa {ED25519}\n"));
    assert!(hosts.pins(&target("a@mac")).is_err());
    // Damage under another name does not affect this host.
    let (_dir, hosts) = file_with(&format!(
        "other ssh-ed25519 !!!\nmac ssh-ed25519 {ED25519}\n"
    ));
    assert_eq!(
        hosts.pins(&target("a@mac")).unwrap(),
        Pins::Keys(vec![ed25519()])
    );
}

#[test]
fn trust_writes_plain_lines_and_keeps_other_hosts() {
    let (_dir, hosts) = file_with(&format!("other ssh-ed25519 {ED25519}"));
    hosts
        .trust(&target("a@mac:2222"), &[ecdsa(), ecdsa(), ed25519()])
        .unwrap();
    assert_eq!(
        contents(&hosts),
        format!(
            "other ssh-ed25519 {ED25519}\n\
             [mac]:2222 ecdsa-sha2-nistp256 {ECDSA}\n\
             [mac]:2222 ssh-ed25519 {ED25519}\n"
        )
    );
    assert_eq!(
        hosts.pins(&target("a@mac:2222")).unwrap(),
        Pins::Keys(vec![ecdsa(), ed25519()])
    );
}

#[test]
fn trust_creates_a_private_file_and_directory() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Application Support").join("known_hosts");
    let hosts = KnownHosts::new(&path);
    hosts.trust(&target("a@mac"), &[ed25519()]).unwrap();
    assert_eq!(contents(&hosts), format!("mac ssh-ed25519 {ED25519}\n"));
    let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let parent = fs::metadata(path.parent().unwrap()).unwrap();
    assert_eq!(parent.permissions().mode() & 0o777, 0o700);
    let leftovers: Vec<_> = fs::read_dir(path.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(leftovers, [OsString::from("known_hosts")]);
}

#[test]
fn trust_refuses_a_host_that_gained_a_pin() {
    let original = format!("mac ecdsa-sha2-nistp256 {ECDSA}\n");
    let (_dir, hosts) = file_with(&original);
    // A key of another type is refused too: any pin blocks a new one.
    assert!(matches!(
        hosts.trust(&target("a@mac"), &[ed25519()]),
        Err(Error::PinnedMeanwhile)
    ));
    assert_eq!(contents(&hosts), original);
}

#[test]
fn trust_needs_a_key() {
    let (_dir, hosts) = file_with("");
    assert!(matches!(
        hosts.trust(&target("a@mac"), &[]),
        Err(Error::Io(error)) if error.kind() == io::ErrorKind::InvalidInput
    ));
    assert_eq!(contents(&hosts), "");
}

#[test]
fn concurrent_trusts_pin_once() {
    let (_dir, hosts) = file_with("");
    let threads = 8;
    let barrier = Arc::new(Barrier::new(threads));
    let workers: Vec<_> = (0..threads)
        .map(|index| {
            let hosts = hosts.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let key = if index % 2 == 0 { ed25519() } else { ecdsa() };
                barrier.wait();
                hosts.trust(&target("a@mac"), &[key])
            })
        })
        .collect();
    let outcomes: Vec<_> = workers
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    assert!(
        outcomes
            .iter()
            .filter_map(|outcome| outcome.as_ref().err())
            .all(|error| matches!(error, Error::PinnedMeanwhile))
    );
    assert_eq!(contents(&hosts).lines().count(), 1);
}

#[test]
fn forget_removes_only_the_name() {
    let (_dir, hosts) = file_with(&format!(
        "# keep\n\
         mac ssh-ed25519 {ED25519}\n\
         [mac]:2222 ssh-ed25519 {ED25519}\n\
         MAC ecdsa-sha2-nistp256 {ECDSA}\n\
         other,mac ssh-ed25519 {ED25519} note\n\
         mac,!other ssh-ed25519 {ED25519}\n\
         mac ssh-ed25519 !!!\n"
    ));
    hosts.forget(&target("a@mac")).unwrap();
    assert_eq!(
        contents(&hosts),
        format!(
            "# keep\n\
             [mac]:2222 ssh-ed25519 {ED25519}\n\
             other ssh-ed25519 {ED25519} note\n"
        )
    );
    assert_eq!(hosts.pins(&target("a@mac")).unwrap(), Pins::None);
    assert_eq!(
        hosts.pins(&target("a@other")).unwrap(),
        Pins::Keys(vec![ed25519()])
    );
}

#[test]
fn forget_without_a_file_or_pin_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let hosts = KnownHosts::new(dir.path().join("absent"));
    hosts.forget(&target("a@mac")).unwrap();
    assert!(!dir.path().join("absent").exists());
    let (_dir, hosts) = file_with("other ssh-ed25519 x\n");
    hosts.forget(&target("a@mac")).unwrap();
    assert_eq!(contents(&hosts), "other ssh-ed25519 x\n");
}

#[test]
fn oversized_file_is_refused() {
    let (_dir, hosts) = file_with(&"#".repeat(FILE_LIMIT + 1));
    assert!(matches!(
        hosts.pins(&target("a@mac")),
        Err(Error::KnownHosts {
            operation: "is too large",
            ..
        })
    ));
}

#[test]
fn truncated_pin_fails_closed_and_can_be_removed() {
    for damaged in ["mac", "mac ssh-ed25519"] {
        let (_dir, hosts) = file_with(damaged);
        assert!(matches!(
            hosts.pins(&target("a@mac")),
            Err(Error::KnownHosts { .. })
        ));
        assert!(hosts.trust(&target("a@mac"), &[ed25519()]).is_err());
        assert_eq!(contents(&hosts), damaged);
        hosts.forget(&target("a@mac")).unwrap();
        assert_eq!(contents(&hosts), "");
    }
}

#[test]
fn trust_cannot_grow_the_file_past_its_read_limit() {
    let original = format!("#{}", "x".repeat(FILE_LIMIT - 2));
    let (_dir, hosts) = file_with(&original);
    assert!(matches!(
        hosts.trust(&target("a@mac"), &[ed25519()]),
        Err(Error::KnownHosts {
            operation: "is too large",
            ..
        })
    ));
    assert_eq!(contents(&hosts), original);
}

#[test]
fn forgetting_requires_the_pins_the_user_reviewed() {
    let original = format!("mac ssh-ed25519 {ED25519}\n");
    let (_dir, hosts) = file_with(&original);
    let target = target("a@mac");
    let reviewed = hosts.pins(&target).unwrap();
    fs::write(&hosts.path, format!("mac ecdsa-sha2-nistp256 {ECDSA}\n")).unwrap();
    let changed = contents(&hosts);
    assert!(matches!(
        hosts.forget_if_matches(&target, &reviewed),
        Err(Error::PinnedMeanwhile)
    ));
    assert_eq!(contents(&hosts), changed);
    let reviewed = hosts.pins(&target).unwrap();
    hosts.forget_if_matches(&target, &reviewed).unwrap();
    assert_eq!(hosts.pins(&target).unwrap(), Pins::None);
}
