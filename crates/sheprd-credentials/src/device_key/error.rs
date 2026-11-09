#[cfg(target_os = "ios")]
use core_foundation::error::CFError;
#[cfg(target_os = "ios")]
use security_framework::base::Error as Status;

/// Why the device key could not be found, created, or used. The display
/// text is shown to the user and carries no key material.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    // keep-sorted start
    #[cfg(target_os = "ios")]
    #[error("The device key's access control could not be set up.")]
    AccessControl(#[source] Status),
    #[cfg(target_os = "ios")]
    #[error("The device key could not be created.")]
    Create(#[source] Status),
    #[error("The Keychain item under the device key's label is not a key.")]
    Item,
    #[cfg(target_os = "ios")]
    #[error("The device key could not be read from or removed from the Keychain.")]
    Keychain(#[source] Status),
    /// `errSecInteractionNotAllowed`: the key's access class keeps it out of
    /// reach until the device is unlocked. Transient, and never a reason to
    /// create another key.
    #[error("The iPad is locked. Unlock it and try again.")]
    Locked,
    #[error("The device key returned a malformed signature.")]
    MalformedSignature,
    #[error("The device key was not in the Keychain after it was created. Try again.")]
    Missing,
    #[error("The device key has no P-256 public key.")]
    PublicKey,
    #[cfg(target_os = "ios")]
    #[error("The device key could not sign.")]
    Sign(#[source] Status),
    // keep-sorted end
}

#[cfg(target_os = "ios")]
impl Error {
    /// A locked device is its own error, so callers can wait for an unlock.
    pub(super) fn from_status(status: Status, other: fn(Status) -> Self) -> Self {
        if status.code() == crate::sec_item::LOCKED {
            Self::Locked
        } else {
            other(status)
        }
    }

    /// Security.framework reports key failures as a `CFError` whose code is
    /// the `OSStatus`. Only the code is kept: a `CFError` cannot cross
    /// threads.
    pub(super) fn from_cf(error: &CFError, other: fn(Status) -> Self) -> Self {
        let code = i32::try_from(error.code()).unwrap_or(security_framework_sys::base::errSecParam);
        Self::from_status(Status::from_code(code), other)
    }
}
