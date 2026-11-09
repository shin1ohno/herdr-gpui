use super::*;

fn key(kind: &str, bits: u32) -> HostKeyFingerprint {
    HostKeyFingerprint {
        kind: kind.into(),
        bits,
        fingerprint: "SHA256:example".into(),
    }
}

#[test]
fn a_key_without_a_reported_size_shows_only_its_type() {
    assert_eq!(key_label(&key("RSA", 0)), "RSA");
    assert_eq!(key_label(&key("ED25519", 256)), "ED25519 256");
}

#[test]
fn an_unrecognized_key_type_points_at_the_default_host_file() {
    let text =
        key_check(&[key("sk-ssh-ed25519@openssh.com", 0)]).replace(['\u{2068}', '\u{2069}'], "");
    assert!(text.contains("/etc/ssh/ssh_host_ed25519_key.pub"));
    assert!(!text.contains("sk-ssh-ed25519"));
    let ecdsa = key_check(&[key("ECDSA", 256)]).replace(['\u{2068}', '\u{2069}'], "");
    assert!(ecdsa.contains("/etc/ssh/ssh_host_ecdsa_key.pub"));
}
