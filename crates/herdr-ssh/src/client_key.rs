//! The client's P-256 key, held outside this crate. On iPad the private key
//! lives in the Secure Enclave and never enters the process; tests and the
//! simulator use a software key behind the same closure.

use std::sync::Arc;

use thiserror::Error;

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
        _public_sec1: impl Into<Arc<[u8]>>,
        _sign: impl Fn(&[u8]) -> Result<[u8; 64], SignError> + Send + Sync + 'static,
    ) -> crate::Result<Self> {
        todo!("ssh stream")
    }

    /// The line for `~/.ssh/authorized_keys`:
    /// `restrict ecdsa-sha2-nistp256 AAAA… <comment>`.
    pub fn authorized_keys_line(&self, _comment: &str) -> String {
        todo!("ssh stream")
    }
}
