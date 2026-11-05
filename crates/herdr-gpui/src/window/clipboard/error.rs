//! Clipboard failures shared by native readers and preparation.

use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // keep-sorted start
    #[cfg(target_os = "ios")]
    #[error("Clipboard acquisition was cancelled.")]
    Cancelled,
    #[error("The clipboard changed while reading. Try pasting again.")]
    Changed,
    #[error("Clipboard text is not valid UTF-8.")]
    Encoding(#[source] std::str::Utf8Error),
    #[cfg(target_os = "ios")]
    #[error("Clipboard image exceeds the {limit}-pixel decode limit.")]
    ImageDimensions { limit: u64 },
    #[cfg(target_os = "ios")]
    #[error("Clipboard image object could not be encoded safely.")]
    ImageObject,
    #[cfg(target_os = "macos")]
    #[error(transparent)]
    Native(#[from] super::macos::Error),
    #[error(
        "Could not read the clipboard; on Linux, install wl-clipboard for Wayland or xclip for X11."
    )]
    Process(#[source] io::Error),
    #[cfg(target_os = "ios")]
    #[error("Could not load clipboard content.")]
    Provider(#[source] objc2::rc::Retained<objc2_foundation::NSError>),
    #[cfg(target_os = "ios")]
    #[error("Previous native clipboard requests are still finishing.")]
    ProviderBusy,
    #[cfg(target_os = "ios")]
    #[error("The clipboard provider closed before returning content.")]
    ProviderClosed,
    #[cfg(target_os = "ios")]
    #[error("The clipboard provider returned an unexpected object type.")]
    ProviderObject,
    #[cfg(target_os = "ios")]
    #[error("The clipboard contains more than {limit} providers.")]
    Providers { limit: usize },
    #[error("Clipboard content exceeds the {limit}-byte limit.")]
    Size { limit: usize },
    #[error("Clipboard acquisition timed out.")]
    Timeout,
    #[error("Remote clipboard acquisition is not supported on this platform.")]
    Unsupported,
    // keep-sorted end
}
