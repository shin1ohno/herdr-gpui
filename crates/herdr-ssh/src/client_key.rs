//! The client's P-256 key, held outside this crate. On iPad the private key
//! lives in the Secure Enclave and never enters the process; tests and the
//! simulator use a software key behind the same closure.

use std::{future::Future, sync::Arc};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use russh::keys::{HashAlg, PublicKey, agent::AgentIdentity};
use thiserror::Error;

use crate::{Error, host_key::write_string};

/// The SSH name of the key and of its signatures.
const ALGORITHM: &str = "ecdsa-sha2-nistp256";
const CURVE: &str = "nistp256";
/// An uncompressed SEC1 point: the `0x04` tag, then 32-byte X and Y.
const SEC1_LENGTH: usize = 65;
const SEC1_UNCOMPRESSED: u8 = 0x04;

/// The holder of the private key refused or failed to sign.
#[derive(Debug, Error)]
#[error("the device key could not sign: {reason}")]
pub struct SignError {
    pub reason: &'static str,
}

type Sign = dyn Fn(&[u8]) -> Result<[u8; 64], SignError> + Send + Sync;

/// An `ecdsa-sha2-nistp256` client key: its public half and a closure that
/// signs a message with the private half, returning the signature as
/// `r || s` (IEEE P1363, 64 bytes). The closure hashes the message itself
/// (ECDSA with SHA-256), as `SecKeyCreateSignature` with
/// `ecdsaSignatureMessageX962SHA256` does.
#[derive(Clone)]
pub struct ClientKey {
    public_sec1: Arc<[u8]>,
    sign: Arc<Sign>,
}

impl std::fmt::Debug for ClientKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientKey").finish_non_exhaustive()
    }
}

impl ClientKey {
    /// `public_sec1` is the 65-byte uncompressed SEC1 point.
    pub fn new(
        public_sec1: impl Into<Arc<[u8]>>,
        sign: impl Fn(&[u8]) -> Result<[u8; 64], SignError> + Send + Sync + 'static,
    ) -> crate::Result<Self> {
        let public_sec1 = public_sec1.into();
        if public_sec1.len() != SEC1_LENGTH || public_sec1.first() != Some(&SEC1_UNCOMPRESSED) {
            return Err(Error::InvalidClientKey {
                reason: "the public key is not a 65-byte uncompressed P-256 point",
            });
        }
        let key = Self {
            public_sec1,
            sign: Arc::new(sign),
        };
        // The point is checked on the curve when the server verifies a
        // signature; here only the encoding russh sends is checked.
        key.public_key()?;
        Ok(key)
    }

    /// The line for `~/.ssh/authorized_keys`:
    /// `restrict ecdsa-sha2-nistp256 AAAA… <comment>`. `restrict` turns off
    /// the PTY and every forwarding, which the bridge never uses. Characters
    /// of `comment` other than visible ASCII become `-`, so the comment can
    /// neither break the line nor start another.
    pub fn authorized_keys_line(&self, comment: &str) -> String {
        let mut line = format!("restrict {ALGORITHM} {}", STANDARD.encode(self.blob()));
        if !comment.is_empty() {
            line.push(' ');
            line.extend(
                comment
                    .chars()
                    .map(|c| if c.is_ascii_graphic() { c } else { '-' }),
            );
        }
        line
    }

    /// The key's SSH wire encoding.
    pub(crate) fn blob(&self) -> Vec<u8> {
        let mut blob = Vec::with_capacity(4 + ALGORITHM.len() + 4 + CURVE.len() + 4 + SEC1_LENGTH);
        write_string(&mut blob, ALGORITHM.as_bytes());
        write_string(&mut blob, CURVE.as_bytes());
        write_string(&mut blob, &self.public_sec1);
        blob
    }

    /// The key as russh offers it to the server.
    pub(crate) fn public_key(&self) -> crate::Result<PublicKey> {
        PublicKey::from_bytes(&self.blob()).map_err(|_| Error::InvalidClientKey {
            reason: "the public key is not a valid P-256 point encoding",
        })
    }

    pub(crate) fn signer(&self) -> DeviceSigner {
        DeviceSigner {
            sign: Arc::clone(&self.sign),
        }
    }
}

/// Signs russh's public-key authentication request with the closure.
pub(crate) struct DeviceSigner {
    sign: Arc<Sign>,
}

/// Why signing the authentication request failed.
#[derive(Debug, Error)]
pub(crate) enum SignerError {
    #[error(transparent)]
    Send(#[from] russh::SendError),
    #[error(transparent)]
    Sign(#[from] SignError),
    #[error("the signing task ended without a result")]
    Join(#[from] tokio::task::JoinError),
}

impl russh::Signer for DeviceSigner {
    type Error = SignerError;

    /// Returns `to_sign` followed by the signature as an SSH `string`, the
    /// shape russh's own agent signer returns.
    fn auth_sign(
        &mut self,
        _key: &AgentIdentity,
        _hash_alg: Option<HashAlg>,
        to_sign: Vec<u8>,
    ) -> impl Future<Output = Result<Vec<u8>, Self::Error>> + Send {
        let sign = Arc::clone(&self.sign);
        async move {
            // The closure may wait on the Secure Enclave, so it runs on the
            // blocking pool rather than the runtime's only thread.
            let (mut signed, signature) = tokio::task::spawn_blocking(move || {
                let signature = sign(&to_sign);
                (to_sign, signature)
            })
            .await?;
            write_string(&mut signed, &signature_blob(&signature?));
            Ok(signed)
        }
    }
}

/// An `ecdsa-sha2-nistp256` signature in SSH form (RFC 5656 section 3.1.2):
/// the algorithm name, then `mpint r || mpint s` as one string.
pub(crate) fn signature_blob(r_s: &[u8; 64]) -> Vec<u8> {
    let (r, s) = r_s.split_at(32);
    let mut scalars = Vec::with_capacity(2 * (4 + 33));
    write_mpint(&mut scalars, r);
    write_mpint(&mut scalars, s);
    let mut blob = Vec::with_capacity(4 + ALGORITHM.len() + 4 + scalars.len());
    write_string(&mut blob, ALGORITHM.as_bytes());
    write_string(&mut blob, &scalars);
    blob
}

/// Appends the unsigned big-endian `magnitude` as an SSH `mpint`: no leading
/// zero bytes, except one when the high bit would otherwise read as a sign.
fn write_mpint(out: &mut Vec<u8>, magnitude: &[u8]) {
    let start = magnitude
        .iter()
        .position(|&byte| byte != 0)
        .unwrap_or(magnitude.len());
    let trimmed = magnitude.get(start..).unwrap_or_default();
    if trimmed.first().is_some_and(|&byte| byte & 0x80 != 0) {
        let mut padded = Vec::with_capacity(trimmed.len() + 1);
        padded.push(0);
        padded.extend_from_slice(trimmed);
        write_string(out, &padded);
    } else {
        write_string(out, trimmed);
    }
}

#[cfg(test)]
pub(crate) mod tests;
