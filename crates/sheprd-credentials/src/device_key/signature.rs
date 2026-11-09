//! ECDSA P-256 signatures from the DER form `SecKeyCreateSignature` returns
//! (`ECDSA-Sig-Value ::= SEQUENCE { r INTEGER, s INTEGER }`) to the fixed
//! 64-byte `r || s` that SSH's `ecdsa-sha2-nistp256` encoding starts from.

use super::Error;

/// Each of `r` and `s` is a P-256 scalar: at most 32 bytes.
const SCALAR: usize = 32;

/// The signature `der` as `r || s`, each left-padded to 32 bytes. Only
/// strict DER is accepted: short-form lengths, minimal positive non-zero
/// integers of at most 32 bytes, and nothing after the sequence.
pub(crate) fn p1363(der: &[u8]) -> Result<[u8; 2 * SCALAR], Error> {
    let body = match der {
        [0x30, length, body @ ..] if *length < 0x80 && usize::from(*length) == body.len() => body,
        _ => return Err(Error::MalformedSignature),
    };
    let (r, rest) = integer(body)?;
    let (s, rest) = integer(rest)?;
    if !rest.is_empty() {
        return Err(Error::MalformedSignature);
    }
    let mut signature = [0; 2 * SCALAR];
    let (r_out, s_out) = signature.split_at_mut(SCALAR);
    r_out[SCALAR - r.len()..].copy_from_slice(r);
    s_out[SCALAR - s.len()..].copy_from_slice(s);
    Ok(signature)
}

/// The DER INTEGER at the start of `bytes` as its big-endian magnitude, and
/// the bytes after it.
fn integer(bytes: &[u8]) -> Result<(&[u8], &[u8]), Error> {
    let [0x02, length, rest @ ..] = bytes else {
        return Err(Error::MalformedSignature);
    };
    let length = usize::from(*length);
    if length >= 0x80 || length > rest.len() {
        return Err(Error::MalformedSignature);
    }
    let (value, rest) = rest.split_at(length);
    let magnitude = match value {
        // A leading zero only marks a high first byte as positive.
        [0x00, next, ..] if next & 0x80 != 0 => &value[1..],
        // Empty, zero, a redundant leading zero, or negative.
        [] | [0x00, ..] => return Err(Error::MalformedSignature),
        [first, ..] if first & 0x80 != 0 => return Err(Error::MalformedSignature),
        _ => value,
    };
    if magnitude.len() > SCALAR {
        return Err(Error::MalformedSignature);
    }
    Ok((magnitude, rest))
}
