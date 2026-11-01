use super::*;
use crate::device_key::signature::p1363;
use p256::ecdsa::{Signature, SigningKey, signature::Signer};

/// A DER `ECDSA-Sig-Value` whose INTEGERs hold exactly `r` and `s`.
fn der(r: &[u8], s: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    for value in [r, s] {
        body.extend([0x02, u8::try_from(value.len()).unwrap()]);
        body.extend(value);
    }
    let mut der = vec![0x30, u8::try_from(body.len()).unwrap()];
    der.extend(body);
    der
}

fn malformed(der: &[u8]) -> bool {
    matches!(p1363(der), Err(Error::MalformedSignature))
}

#[test]
fn full_width_scalars_are_copied_as_they_are() {
    let (r, s) = ([0x11; 32], [0x7f; 32]);
    let signature = p1363(&der(&r, &s)).unwrap();
    assert_eq!(signature[..32], r);
    assert_eq!(signature[32..], s);
}

/// DER prefixes a zero byte when a scalar's top bit is set, so it does not
/// read as negative; the fixed-width form has no sign byte.
#[test]
fn a_sign_byte_is_removed() {
    let (r, s) = ([0x80; 32], [0xff; 32]);
    let signature = p1363(&der(&[&[0][..], &r].concat(), &[&[0][..], &s].concat())).unwrap();
    assert_eq!(signature[..32], r);
    assert_eq!(signature[32..], s);
}

/// DER drops leading zero bytes; the fixed-width form puts them back.
#[test]
fn short_scalars_are_left_padded() {
    let r = [0x42; 31];
    let signature = p1363(&der(&r, &[0x01])).unwrap();
    assert_eq!(signature[0], 0);
    assert_eq!(signature[1..32], r);
    assert_eq!(signature[32..63], [0; 31]);
    assert_eq!(signature[63], 0x01);
    // A short scalar whose top bit is set keeps its sign byte in DER.
    let signature = p1363(&der(&[0x00, 0x80], &[0x05])).unwrap();
    assert_eq!(signature[30..32], [0x00, 0x80]);
    assert_eq!(signature[..30], [0; 30]);
}

#[test]
fn malformed_sequences_are_refused() {
    let valid = der(&[0x01], &[0x02]);
    assert!(p1363(&valid).is_ok());
    assert!(malformed(&[]));
    assert!(malformed(&[0x30]));
    // Not a SEQUENCE.
    assert!(malformed(&[&[0x31][..], &valid[1..]].concat()));
    // A length longer or shorter than what follows.
    assert!(malformed(&valid[..valid.len() - 1]));
    assert!(malformed(&[&valid[..], &[0x00]].concat()));
    // Long-form length, which DER forbids for a length under 128.
    assert!(malformed(&[&[0x30, 0x81, 0x06][..], &valid[2..]].concat()));
    // Bytes left after `s` inside the sequence.
    let mut extra = valid.clone();
    extra[1] += 3;
    extra.extend([0x02, 0x01, 0x03]);
    assert!(malformed(&extra));
    // `s` missing.
    assert!(malformed(&[0x30, 0x03, 0x02, 0x01, 0x01]));
}

#[test]
fn malformed_integers_are_refused() {
    let one = [0x01];
    // Not an INTEGER.
    let mut bit_string = der(&one, &one);
    bit_string[2] = 0x03;
    assert!(malformed(&bit_string));
    // Empty, zero, negative, or with a redundant leading zero.
    assert!(malformed(&der(&[], &one)));
    assert!(malformed(&der(&[0x00], &one)));
    assert!(malformed(&der(&one, &[0x80])));
    assert!(malformed(&der(&[0x00, 0x01], &one)));
    // Wider than a P-256 scalar, with or without a sign byte.
    assert!(malformed(&der(&[0x01; 33], &one)));
    assert!(malformed(&der(&[&[0][..], &[0x80; 33]].concat(), &one)));
    // A length that runs past the sequence.
    let mut overrun = der(&one, &one);
    overrun[3] = 0x05;
    assert!(malformed(&overrun));
}

/// Real signatures convert to exactly the fixed-width form a P-256
/// implementation produces for them.
#[test]
fn real_signatures_convert_to_their_fixed_width_form() {
    for seed in 1..=200u8 {
        let key = SigningKey::from_slice(&[seed; 32]).unwrap();
        let message = [b"herdr device key ".as_slice(), &[seed]].concat();
        let signature: Signature = key.sign(&message);
        let encoded = signature.to_der();
        assert_eq!(
            p1363(encoded.as_bytes()).unwrap()[..],
            signature.to_bytes()[..]
        );
    }
}
