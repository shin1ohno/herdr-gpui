use crate::sec_item::LOCKED;
use security_framework::base::Error as Status;

/// Why the Keychain refused a saved sign-in, with its status as the source.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    // keep-sorted start
    #[error("The iPad is locked. Unlock it to use saved sign-ins.")]
    Locked,
    #[error("Cannot read a saved sign-in from the Keychain. Sign in again.")]
    Read(#[source] Status),
    #[error("The Keychain returned a saved sign-in that is not data. Sign in again.")]
    Value,
    #[error("Saving the sign-in to the Keychain failed. Try again.")]
    Write(#[source] Status),
    // keep-sorted end
}

impl Error {
    /// A locked device is its own error, so callers can wait for an unlock.
    pub(super) fn from_status(status: Status, other: fn(Status) -> Self) -> Self {
        if status.code() == LOCKED {
            Self::Locked
        } else {
            other(status)
        }
    }
}
