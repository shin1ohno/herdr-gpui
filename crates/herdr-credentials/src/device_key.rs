//! The iPad's own P-256 key, which signs SSH authentication (docs/ios.md,
//! "Client key"). On a device the private key is created in the Secure
//! Enclave and never leaves it; the simulator has no Secure Enclave, so there
//! it is a software key kept in the Keychain with the same access class. The
//! Keychain item is `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`, not
//! synchronizable, and in no shared access group. It is created only when a
//! lookup finds none, and every caller then uses the key the Keychain holds.
//!
//! The signature conversion and the lookup-or-create rule are plain Rust, so
//! their tests also run on the build host.

mod create;
mod error;
#[cfg(target_os = "ios")]
mod keychain;
mod signature;

pub use error::Error;

#[cfg(target_os = "ios")]
use security_framework::key::SecKey;
#[cfg(target_os = "ios")]
use std::sync::{Mutex, PoisonError};

/// Held while looking up, creating, or deleting the key, so two scenes that
/// ask at once both end with the key the first one created. The Keychain
/// does not refuse a second key under the same tag (each key is its own
/// item), so this lock is what keeps it to one; the item is in no shared
/// access group, so no other process can add one under the label.
#[cfg(target_os = "ios")]
static KEYCHAIN: Mutex<()> = Mutex::new(());

/// A handle to the installation's device key. Every call may block on the
/// Keychain or the Secure Enclave: call it from a background worker.
#[cfg(target_os = "ios")]
#[derive(Debug, Clone)]
pub struct DeviceKey {
    private: SecKey,
    public: [u8; 65],
}

#[cfg(target_os = "ios")]
impl DeviceKey {
    /// The key stored under `label`, created on first use. A locked device
    /// is a transient error, never a reason to create a new key.
    pub fn load_or_create(label: &str) -> crate::Result<Self> {
        let _keychain = KEYCHAIN.lock().unwrap_or_else(PoisonError::into_inner);
        let private = create::find_or_create(|| keychain::find(label), || keychain::create(label))?;
        let public = keychain::public_sec1(&private)?;
        Ok(Self { private, public })
    }

    /// Deletes the key under `label`, so the next `load_or_create` makes a
    /// new one; the old public key must then be removed from hosts.
    pub fn delete(label: &str) -> crate::Result<()> {
        let _keychain = KEYCHAIN.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(keychain::delete(label)?)
    }

    /// The 65-byte uncompressed SEC1 public point.
    pub fn public_sec1(&self) -> [u8; 65] {
        self.public
    }

    /// ECDSA P-256 over SHA-256 of `message`, as `r || s` (64 bytes).
    pub fn sign(&self, message: &[u8]) -> crate::Result<[u8; 64]> {
        let der = keychain::sign(&self.private, message)?;
        Ok(signature::p1363(&der)?)
    }
}

#[cfg(test)]
mod tests;
