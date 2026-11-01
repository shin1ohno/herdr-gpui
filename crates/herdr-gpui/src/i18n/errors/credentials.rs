//! `herdr_credentials::Error`: reading or writing a saved sign-in, and on
//! Windows the Credential Manager failure it wraps, from
//! `error-credentials.ftl`. No message carries the secret.

use super::Localize;
use crate::i18n::fl;
use herdr_credentials::Error;

impl Localize for Error {
    fn localize(&self) -> String {
        match self {
            // keep-sorted start
            #[cfg(windows)]
            Self::CredentialManager(error) => error.localize(),
            #[cfg(target_os = "ios")]
            Self::DeviceKey(error) => error.localize(),
            Self::Encoding(_) => fl!("error-credentials-encoding"),
            Self::Io(_) => fl!("error-credentials-io"),
            #[cfg(target_os = "ios")]
            Self::IosKeychain(error) => error.localize(),
            #[cfg(target_os = "macos")]
            Self::KeychainRead(_) => fl!("error-credentials-keychain-read"),
            #[cfg(target_os = "macos")]
            Self::KeychainWrite(_) => fl!("error-credentials-keychain-write"),
            Self::Permissions => fl!("error-credentials-permissions"),
            Self::Policy => fl!("error-credentials-policy"),
            #[cfg(target_os = "linux")]
            Self::SecretServiceRead(_) => fl!("error-credentials-secret-service-read"),
            #[cfg(target_os = "linux")]
            Self::SecretServiceUnavailable => fl!("error-credentials-secret-service-unavailable"),
            #[cfg(target_os = "linux")]
            Self::SecretServiceWrite(_) => fl!("error-credentials-secret-service-write"),
            Self::Unsupported => fl!("error-credentials-unsupported"),
            // keep-sorted end
        }
    }
}

#[cfg(windows)]
impl Localize for herdr_credentials::CredentialManagerError {
    fn localize(&self) -> String {
        match self {
            Self::Read(_) => fl!("error-credentials-credential-manager-read"),
            Self::Write(_) => fl!("error-credentials-credential-manager-write"),
        }
    }
}

#[cfg(target_os = "ios")]
impl Localize for herdr_credentials::device_key::Error {
    fn localize(&self) -> String {
        match self {
            // keep-sorted start
            Self::AccessControl(_) => fl!("error-credentials-device-key-access-control"),
            Self::Create(_) => fl!("error-credentials-device-key-create"),
            Self::Item => fl!("error-credentials-device-key-item"),
            Self::Keychain(_) => fl!("error-credentials-device-key-keychain"),
            Self::Locked => fl!("error-credentials-device-key-locked"),
            Self::MalformedSignature => fl!("error-credentials-device-key-malformed-signature"),
            Self::Missing => fl!("error-credentials-device-key-missing"),
            Self::PublicKey => fl!("error-credentials-device-key-public-key"),
            Self::Sign(_) => fl!("error-credentials-device-key-sign"),
            // keep-sorted end
        }
    }
}

#[cfg(target_os = "ios")]
impl Localize for herdr_credentials::ios_keychain::Error {
    fn localize(&self) -> String {
        match self {
            // keep-sorted start
            Self::Locked => fl!("error-credentials-ios-keychain-locked"),
            Self::Read(_) => fl!("error-credentials-ios-keychain-read"),
            Self::Value => fl!("error-credentials-ios-keychain-value"),
            Self::Write(_) => fl!("error-credentials-ios-keychain-write"),
            // keep-sorted end
        }
    }
}
