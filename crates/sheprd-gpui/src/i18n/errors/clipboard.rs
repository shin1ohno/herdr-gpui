//! Clipboard failures, from `error-clipboard.ftl`.

use super::Localize;
use crate::{i18n::fl, window::clipboard::Error};

impl Localize for Error {
    fn localize(&self) -> String {
        match self {
            #[cfg(target_os = "ios")]
            Self::Cancelled => fl!("error-clipboard-cancelled"),
            Self::Changed => fl!("error-clipboard-changed"),
            Self::Encoding(_) => fl!("error-clipboard-encoding"),
            #[cfg(target_os = "macos")]
            Self::Native(error) => error.localize(),
            Self::Process(_) => fl!("error-clipboard-process"),
            #[cfg(target_os = "ios")]
            Self::ImageDimensions { .. }
            | Self::ImageObject
            | Self::Provider(_)
            | Self::ProviderBusy
            | Self::ProviderClosed
            | Self::ProviderObject
            | Self::Providers { .. } => {
                fl!("error-clipboard-provider")
            }
            Self::Size { limit } => fl!("error-clipboard-size", limit = *limit),
            Self::Timeout => fl!("error-clipboard-timeout"),
            Self::Unsupported => fl!("error-clipboard-unsupported"),
        }
    }
}

#[cfg(target_os = "macos")]
impl Localize for crate::window::clipboard::macos::Error {
    fn localize(&self) -> String {
        match self {
            // keep-sorted start
            // The exit status as the system writes it.
            Self::Exited(status) => {
                fl!("error-clipboard-macos-exited", status = status.to_string())
            }
            Self::Missing(_) => fl!("error-clipboard-macos-missing"),
            Self::Process(_) => fl!("error-clipboard-macos-process"),
            Self::Response => fl!("error-clipboard-macos-response"),
            Self::TextEncoding => fl!("error-clipboard-macos-text-encoding"),
            // keep-sorted end
        }
    }
}
