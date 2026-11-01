//! A host's public key as the wire carries it, compared byte for byte.

use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
use sha2::{Digest as _, Sha256};

/// The longest key type name accepted from a blob or a `known_hosts` line;
/// OpenSSH's longest is under 64 characters.
const ALGORITHM_LIMIT: usize = 64;

/// One host public key: its algorithm name (`ssh-ed25519`,
/// `ecdsa-sha2-nistp256`, `rsa-sha2-512` keys are stored as `ssh-rsa`) and
/// the SSH wire encoding of the key. Equality is exact byte equality.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HostKey {
    algorithm: String,
    blob: Vec<u8>,
}

impl HostKey {
    pub fn algorithm(&self) -> &str {
        &self.algorithm
    }

    /// `SHA256:<unpadded base64>`, as `ssh-keygen -l` prints it.
    pub fn fingerprint(&self) -> String {
        format!(
            "SHA256:{}",
            STANDARD_NO_PAD.encode(Sha256::digest(&self.blob))
        )
    }

    /// A key from its wire encoding, whose leading string names its type.
    /// `None` for a blob that does not start with a plausible type name.
    pub(crate) fn from_blob(blob: Vec<u8>) -> Option<Self> {
        let (algorithm, _) = read_string(&blob)?;
        let algorithm = std::str::from_utf8(algorithm).ok()?;
        if algorithm.is_empty()
            || algorithm.len() > ALGORITHM_LIMIT
            || !algorithm.bytes().all(|b| b.is_ascii_graphic())
        {
            return None;
        }
        Some(Self {
            algorithm: algorithm.to_owned(),
            blob,
        })
    }

    /// A key from the two `known_hosts` fields after the host names. The type
    /// field must match the type inside the blob.
    pub(crate) fn from_known_hosts(algorithm: &str, base64: &str) -> Option<Self> {
        let key = Self::from_blob(STANDARD.decode(base64).ok()?)?;
        (key.algorithm == algorithm).then_some(key)
    }

    /// The `<type> <base64>` fields of a `known_hosts` line.
    pub(crate) fn known_hosts_fields(&self) -> String {
        format!("{} {}", self.algorithm, STANDARD.encode(&self.blob))
    }
}

/// Reads one SSH `string` (a big-endian `u32` length, then that many bytes)
/// from the front of `bytes`, returning it and the rest.
pub(crate) fn read_string(bytes: &[u8]) -> Option<(&[u8], &[u8])> {
    let (length, rest) = bytes.split_first_chunk::<4>()?;
    let length = usize::try_from(u32::from_be_bytes(*length)).ok()?;
    (length <= rest.len()).then(|| rest.split_at(length))
}

/// Appends `bytes` as an SSH `string`.
pub(crate) fn write_string(out: &mut Vec<u8>, bytes: &[u8]) {
    // Every string this crate writes is far below 4 GiB; saturating keeps the
    // conversion total without a panic path.
    let length = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(bytes);
}

#[cfg(test)]
pub(crate) mod tests;
