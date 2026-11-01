#![allow(clippy::unwrap_used)]

use p256::ecdsa::{Signature, SigningKey, signature::Signer as _};
use russh::Signer as _;
use russh::keys::signature::Verifier;

use super::*;
use crate::host_key::read_string;

/// A software key behind the same closure the Secure Enclave key uses.
pub(crate) fn software_key(seed: u8) -> ClientKey {
    let signing = SigningKey::from_slice(&[seed; 32]).unwrap();
    let public = signing
        .verifying_key()
        .to_sec1_point(false)
        .as_bytes()
        .to_vec();
    ClientKey::new(public, move |message| {
        let signature: Signature = signing.sign(message);
        Ok(signature.to_bytes().into())
    })
    .unwrap()
}

fn block_on<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(future)
}

fn sign(key: &ClientKey, message: &[u8]) -> Result<Vec<u8>, SignerError> {
    let identity = AgentIdentity::PublicKey {
        key: key.public_key().unwrap(),
        comment: String::new(),
    };
    block_on(key.signer().auth_sign(&identity, None, message.to_vec()))
}

#[test]
fn public_key_must_be_uncompressed_sec1() {
    let refused = |bytes: Vec<u8>| {
        matches!(
            ClientKey::new(bytes, |_| Ok([0; 64])),
            Err(Error::InvalidClientKey { .. })
        )
    };
    let good = software_key(1).public_sec1.to_vec();
    assert!(refused(Vec::new()));
    assert!(refused(good[..64].to_vec()));
    assert!(refused([good.as_slice(), &[0]].concat()));
    let mut compressed_tag = good.clone();
    compressed_tag[0] = 0x02;
    assert!(refused(compressed_tag));
    let mut compressed = vec![0x03];
    compressed.extend_from_slice(&good[1..33]);
    assert!(refused(compressed));
    assert!(!refused(good));
}

#[test]
fn authorized_keys_line_is_restricted() {
    let key = software_key(7);
    let openssh = key.public_key().unwrap().to_openssh().unwrap();
    assert!(
        openssh.starts_with(
            "ecdsa-sha2-nistp256 AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTYAAABB"
        )
    );
    assert_eq!(
        key.authorized_keys_line("herdr-ipad-desk"),
        format!("restrict {openssh} herdr-ipad-desk")
    );
    assert_eq!(key.authorized_keys_line(""), format!("restrict {openssh}"));
}

#[test]
fn authorized_keys_comment_cannot_break_the_line() {
    let line = software_key(7).authorized_keys_line("Shin's iPad\nssh-ed25519 AAAA evil\r日本");
    assert_eq!(line.lines().count(), 1);
    assert!(line.ends_with(" Shin's-iPad-ssh-ed25519-AAAA-evil---"));
}

#[test]
fn mpint_encoding() {
    let mpint = |bytes: &[u8]| {
        let mut out = Vec::new();
        write_mpint(&mut out, bytes);
        out
    };
    assert_eq!(mpint(&[0, 0, 0x12, 0x34]), [0, 0, 0, 2, 0x12, 0x34]);
    assert_eq!(mpint(&[0x80, 1]), [0, 0, 0, 3, 0, 0x80, 1]);
    assert_eq!(mpint(&[0, 0xff]), [0, 0, 0, 2, 0, 0xff]);
    assert_eq!(mpint(&[0, 0, 0]), [0, 0, 0, 0]);
    assert_eq!(mpint(&[0x7f]), [0, 0, 0, 1, 0x7f]);
}

#[test]
fn signer_appends_a_signature_the_public_key_verifies() {
    let key = software_key(3);
    let message = b"session id and userauth request";
    let signed = sign(&key, message).unwrap();
    let (prefix, appended) = signed.split_at(message.len());
    assert_eq!(prefix, message);
    let (blob, rest) = read_string(appended).unwrap();
    assert!(rest.is_empty());
    let (algorithm, _) = read_string(blob).unwrap();
    assert_eq!(algorithm, b"ecdsa-sha2-nistp256");
    let signature = russh::keys::ssh_key::Signature::try_from(blob).unwrap();
    let public = key.public_key().unwrap();
    Verifier::verify(&public, message, &signature).unwrap();
    // The same signature does not verify another message.
    assert!(Verifier::verify(&public, b"other", &signature).is_err());
}

#[test]
fn signatures_with_high_bits_and_leading_zeros_stay_valid() {
    // Many signatures exercise both mpint paths: r or s with the high bit
    // set (a padding zero) and, now and then, a leading zero byte.
    let key = software_key(9);
    let public = key.public_key().unwrap();
    for index in 0u32..64 {
        let message = index.to_be_bytes();
        let signed = sign(&key, &message).unwrap();
        let (blob, _) = read_string(&signed[message.len()..]).unwrap();
        let signature = russh::keys::ssh_key::Signature::try_from(blob).unwrap();
        Verifier::verify(&public, &message, &signature).unwrap();
    }
}

#[test]
fn signing_failure_is_typed() {
    let public = software_key(1).public_sec1.to_vec();
    let key = ClientKey::new(public, |_| Err(SignError { reason: "locked" })).unwrap();
    assert!(matches!(
        sign(&key, b"x"),
        Err(SignerError::Sign(SignError { reason: "locked" }))
    ));
}
