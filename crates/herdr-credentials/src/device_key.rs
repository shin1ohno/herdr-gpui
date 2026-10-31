//! The iPad's own P-256 key, which signs SSH authentication (docs/ios.md,
//! "Client key"). On a device the private key is created in the Secure
//! Enclave and never leaves it; the simulator has no Secure Enclave, so there
//! it is a software key kept in the Keychain with the same access class. The
//! Keychain item is `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`, not
//! synchronizable, and in no shared access group. It is created only when a
//! lookup finds none, by an add that fails on a duplicate.

// Wave 0 skeleton for the credentials stream; bodies are `todo!()`.
#![allow(dead_code, clippy::todo)]

use crate::Result;

/// A handle to the installation's device key. Every call may block on the
/// Keychain or the Secure Enclave: call it from a background worker.
#[derive(Debug, Clone)]
pub struct DeviceKey {}

impl DeviceKey {
    /// The key stored under `label`, created on first use. A locked device
    /// is a transient error, never a reason to create a new key.
    pub fn load_or_create(_label: &str) -> Result<Self> {
        todo!("credentials stream")
    }

    /// Deletes the key under `label`, so the next `load_or_create` makes a
    /// new one; the old public key must then be removed from hosts.
    pub fn delete(_label: &str) -> Result<()> {
        todo!("credentials stream")
    }

    /// The 65-byte uncompressed SEC1 public point.
    pub fn public_sec1(&self) -> [u8; 65] {
        todo!("credentials stream")
    }

    /// ECDSA P-256 over SHA-256 of `message`, as `r || s` (64 bytes).
    pub fn sign(&self, _message: &[u8]) -> Result<[u8; 64]> {
        todo!("credentials stream")
    }
}
