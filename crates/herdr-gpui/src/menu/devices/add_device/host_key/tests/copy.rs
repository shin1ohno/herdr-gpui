use super::*;

#[test]
fn ipad_host_key_advice_keeps_the_verification_command_without_a_container_path() {
    let text = key_check(&[HostKeyFingerprint {
        kind: "ED25519".into(),
        bits: 256,
        fingerprint: "SHA256:example".into(),
    }])
    .replace(['\u{2068}', '\u{2069}'], "");
    assert!(text.contains("ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub"));
    assert!(text.contains("Trusting saves this key in the app"));
    assert!(!text.contains("known_hosts"));
    assert!(!text.contains("/Library/"));
    assert!(!text.contains("/Containers/"));
}

#[test]
fn ipad_host_key_advice_names_the_scanned_key_kind() {
    let text = key_check(&[HostKeyFingerprint {
        kind: "RSA".into(),
        bits: 4096,
        fingerprint: "SHA256:example".into(),
    }])
    .replace(['\u{2068}', '\u{2069}'], "");
    assert!(text.contains("ssh-keygen -lf /etc/ssh/ssh_host_rsa_key.pub"));
    assert!(key_check(&[]).contains("/etc/ssh/ssh_host_ed25519_key.pub"));
}
