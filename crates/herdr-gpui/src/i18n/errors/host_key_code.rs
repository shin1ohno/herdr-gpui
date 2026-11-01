//! Host-key-code validation and public-key loading failures.

use super::Localize;
use crate::{host_key_code::Error, i18n::fl};

impl Localize for Error {
    fn localize(&self) -> String {
        match self {
            // keep-sorted start
            Self::Fingerprints => fl!("host-key-code-error-fingerprints"),
            Self::HostHint => fl!("host-key-code-error-host-hint"),
            Self::Invalid => fl!("host-key-code-error-invalid"),
            Self::NoHostKeys => fl!("host-key-code-error-no-host-keys"),
            Self::PublicKey { path } => fl!(
                "host-key-code-error-public-key",
                path = path.display().to_string()
            ),
            Self::Qr(error) => error.localize(),
            Self::Read { path, source } => fl!(
                "host-key-code-error-read",
                path = path.display().to_string(),
                error = source.to_string()
            ),
            Self::TooLarge => fl!("host-key-code-error-too-large"),
            Self::Unsupported => fl!("host-key-code-error-unsupported"),
            Self::Version => fl!("host-key-code-error-version"),
            // keep-sorted end
        }
    }
}
