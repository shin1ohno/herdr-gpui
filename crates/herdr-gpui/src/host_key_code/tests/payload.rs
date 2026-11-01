use super::*;

#[test]
fn payload_round_trip_preserves_all_fingerprints_and_hint() {
    let original = code();
    let encoded = original.to_url();
    assert!(encoded.starts_with("herdr-ipad://host-key?v=1&fp="));
    assert_eq!(Code::parse(&encoded).unwrap(), original);
    assert_eq!(original.fingerprints()[0].algorithm(), "ssh-ed25519");
    assert_eq!(original.host_hint(), Some("computer.local"));
    let mut no_hint = original;
    no_hint.host_hint = None;
    assert_eq!(Code::parse(&no_hint.to_url()).unwrap(), no_hint);
}

#[test]
fn malformed_payloads_are_refused() {
    let valid = code().to_url();
    for input in [
        String::new(),
        valid.replace("herdr-ipad", "https"),
        valid.replace("host-key?", "host-key/?"),
        valid.replace("host-key?", "user@host-key?"),
        valid.replace("host-key?", "host-key:22?"),
        format!("{valid}#fragment"),
        format!("{valid}&unknown=value"),
        format!("{valid}&v=1"),
        format!("{valid}&host=other"),
        format!("{valid}&host="),
        format!("{valid}&"),
        format!("{valid}&host=%"),
        format!("{valid}&host=%0g"),
        valid.replace("&host=computer.local", "&host=%00"),
        valid.replace("&host=computer.local", "&host=two+names"),
        valid.replace("&host=computer.local", "&host=-option"),
        valid.replace("v=1", "v=2"),
        valid.replace("v=1&", ""),
        valid.replace("fp=", "fp"),
        valid.replace("ssh-ed25519", "ssh-dss"),
        valid.replace("SHA256", "SHA1"),
        "herdr-ipad://host-key?v=1".into(),
    ] {
        assert!(Code::parse(&input).is_err(), "accepted {input}");
    }
}

#[test]
fn bounds_and_duplicates_are_refused() {
    assert!(matches!(
        Code::parse(&"a".repeat(MAX_CODE_BYTES + 1)),
        Err(Error::TooLarge)
    ));
    let mut oversized_host = code();
    oversized_host.host_hint = Some("a".repeat(MAX_HOST_BYTES + 1));
    assert!(matches!(
        Code::parse(&oversized_host.to_url()),
        Err(Error::HostHint)
    ));
    let mut duplicate = code();
    duplicate.fingerprints.push(fingerprint("ssh-ed25519", 3));
    assert!(matches!(
        Code::parse(&duplicate.to_url()),
        Err(Error::Fingerprints)
    ));
    let mut identical = code();
    identical
        .fingerprints
        .push(identical.fingerprints[0].clone());
    assert!(matches!(
        Code::parse(&identical.to_url()),
        Err(Error::Fingerprints)
    ));
}

#[test]
fn digests_require_canonical_unpadded_32_byte_sha256() {
    for digest in [
        String::new(),
        "not-base64!".into(),
        STANDARD_NO_PAD.encode([1; 31]),
        STANDARD_NO_PAD.encode([1; 33]),
        format!("{}=", STANDARD_NO_PAD.encode([1; 32])),
        format!("{}B", "A".repeat(42)),
    ] {
        let encoded: String = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("v", "1")
            .append_pair("fp", &format!("ssh-ed25519:SHA256:{digest}"))
            .finish();
        assert!(matches!(
            Code::parse(&format!("{PREFIX}{encoded}")),
            Err(Error::Fingerprints)
        ));
    }
}

#[test]
fn matching_requires_exact_type_curve_and_digest_for_every_scanned_key() {
    let expected = code();
    let offered = HostKeyFingerprint {
        kind: "ED25519".into(),
        bits: 256,
        fingerprint: expected.fingerprints[0].fingerprint.clone(),
    };
    assert!(expected.matches(std::slice::from_ref(&offered)));
    for other in [
        HostKeyFingerprint {
            kind: "ECDSA".into(),
            bits: 256,
            ..offered.clone()
        },
        HostKeyFingerprint {
            bits: 0,
            ..offered.clone()
        },
        HostKeyFingerprint {
            kind: "UNKNOWN".into(),
            ..offered.clone()
        },
        HostKeyFingerprint {
            fingerprint: fingerprint("ssh-ed25519", 9).fingerprint,
            ..offered.clone()
        },
    ] {
        assert!(!expected.matches(std::slice::from_ref(&other)));
        assert!(!expected.matches(&[offered.clone(), other]));
    }
    let ecdsa = HostKeyFingerprint {
        kind: "ECDSA".into(),
        bits: 256,
        fingerprint: expected.fingerprints[1].fingerprint.clone(),
    };
    assert!(expected.matches(std::slice::from_ref(&ecdsa)));
    assert!(!expected.matches(&[HostKeyFingerprint { bits: 384, ..ecdsa }]));
    assert!(!expected.matches(&[]));
}

#[cfg(not(target_os = "ios"))]
#[test]
fn largest_allowed_payload_has_a_scannable_layout_at_the_dialog_size() {
    let code = Code {
        fingerprints: [
            "ssh-ed25519",
            "ecdsa-sha2-nistp256",
            "ecdsa-sha2-nistp384",
            "ecdsa-sha2-nistp521",
            "ssh-rsa",
        ]
        .map(|algorithm| fingerprint(algorithm, 255))
        .to_vec(),
        host_hint: Some("a".repeat(MAX_HOST_BYTES)),
    };
    let url = code.to_url();
    assert!(url.len() <= MAX_CODE_BYTES);
    assert_eq!(Code::parse(&url).unwrap(), code);
    let qr = crate::remote::qr::Qr::encode(&url).unwrap();
    let layout = crate::remote::qr::layout(dialog::QR_SIDE, qr.width()).unwrap();
    assert!(layout.module >= 1.);
}
