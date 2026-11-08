//! Snapshot metadata on the UI thread; load representations only after reservation.

#![allow(
    unsafe_code,
    reason = "main-thread metadata and immutable UIImage properties" // i18n: data
)]

#[path = "ios/request.rs"]
mod request;

use dispatch2::MainThreadBound;
use gpui::{ClipboardItem, ImageFormat};
use objc2::{ClassType, MainThreadMarker, msg_send, rc::Retained};
use objc2_foundation::{NSItemProvider, NSString};
use objc2_ui_kit::{UIImage, UIPasteboard};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use super::{Error, read_with};
use crate::Result;

pub(super) const MAX_PROVIDERS: usize = 32;

#[derive(Clone, Copy)]
enum ObjectKind {
    Image,
    Text,
}

enum Representation {
    Data {
        format: Option<ImageFormat>,
        kind: Retained<NSString>,
    },
    Object(ObjectKind),
}

struct Candidate {
    provider: Retained<NSItemProvider>,
    representation: Representation,
}

/// Contains metadata only. Moving it to a worker starts no native acquisition.
/// NSItemProvider is not Send: MainThreadBound confines its access and release.
pub(crate) struct Read {
    candidates: Arc<MainThreadBound<Vec<Candidate>>>,
    count: usize,
    deadline: Instant,
}

impl Read {
    /// The accepted FIFO preparation worker advances one request at a time.
    /// Nil data and empty images advance; an empty text copy is a valid result.
    pub(crate) fn wait(self, cancelled: impl Fn() -> bool) -> Result<Option<ClipboardItem>> {
        if cancelled() {
            return Err(Error::Cancelled.into());
        }
        if Instant::now() >= self.deadline {
            return Err(Error::Timeout.into());
        }
        for index in 0..self.count {
            if cancelled() {
                return Err(Error::Cancelled.into());
            }
            if Instant::now() >= self.deadline {
                return Err(Error::Timeout.into());
            }
            let loaded = request::load(self.candidates.clone(), index, self.deadline, &cancelled)?;
            let bytes = match loaded.payload {
                Some(request::Payload::Data(bytes)) => Some(bytes),
                Some(request::Payload::Image(image)) => encode_image(&image)?,
                None => None,
            };
            if cancelled() {
                return Err(Error::Cancelled.into());
            }
            if Instant::now() >= self.deadline {
                return Err(Error::Timeout.into());
            }
            let Some(bytes) = bytes else { continue };
            if loaded.format.is_some() && bytes.is_empty() {
                continue;
            }
            let mut bytes = Some(bytes);
            return read_with(loaded.format.is_some(), |format, _| {
                Ok(if format == loaded.format {
                    bytes.take()
                } else {
                    None
                })
            });
        }
        Ok(None)
    }
}

pub(crate) fn read(image_only: bool) -> Result<Read> {
    let main = MainThreadMarker::new().ok_or(Error::Unsupported)?;
    read_pasteboard(&UIPasteboard::generalPasteboard(), image_only, main)
}

pub(super) fn read_pasteboard(
    pasteboard: &UIPasteboard,
    image_only: bool,
    main: MainThreadMarker,
) -> Result<Read> {
    read_pasteboard_until(
        pasteboard,
        image_only,
        main,
        Instant::now() + Duration::from_secs(30),
    )
}

pub(super) fn read_pasteboard_until(
    pasteboard: &UIPasteboard,
    image_only: bool,
    main: MainThreadMarker,
    deadline: Instant,
) -> Result<Read> {
    // SAFETY: only metadata is read, under a main-thread marker. Payload access
    // is deferred until the accepted worker requests an asynchronous load.
    let providers = unsafe { pasteboard.itemProviders() };
    read_providers_until(&providers, image_only, main, deadline)
}

/// The acquisition boundary after the UI-thread pasteboard metadata snapshot.
/// Keeping provider loading separate also lets native regressions distinguish
/// Foundation acquisition from UIPasteboard's eager serialization of publishers.
pub(super) fn read_providers_until(
    providers: &objc2_foundation::NSArray<NSItemProvider>,
    image_only: bool,
    main: MainThreadMarker,
    deadline: Instant,
) -> Result<Read> {
    if providers.len() > MAX_PROVIDERS {
        return Err(Error::Providers {
            limit: MAX_PROVIDERS,
        }
        .into());
    }
    let mut candidates = Vec::new();
    for (format, name) in [
        (None, "public.utf8-plain-text"),
        (Some(ImageFormat::Png), "public.png"),
        (Some(ImageFormat::Jpeg), "public.jpeg"),
        (Some(ImageFormat::Gif), "com.compuserve.gif"),
        (Some(ImageFormat::Webp), "org.webmproject.webp"),
        (Some(ImageFormat::Bmp), "com.microsoft.bmp"),
        (Some(ImageFormat::Tiff), "public.tiff"),
    ] {
        if image_only && format.is_none() {
            continue;
        }
        let kind = NSString::from_str(name);
        for provider in providers.iter() {
            if format.is_none() {
                // NSString-valued copies need the object loader; metadata can
                // advertise UTF-8 while the data-only loader cannot coerce it.
                // SAFETY: documented Foundation class API, on the UI thread.
                let can_load: bool =
                    unsafe { msg_send![&*provider, canLoadObjectOfClass: NSString::class()] };
                if can_load {
                    candidates.push(Candidate {
                        provider: provider.clone(),
                        representation: Representation::Object(ObjectKind::Text),
                    });
                }
            }
            if provider.hasItemConformingToTypeIdentifier(&kind) {
                candidates.push(Candidate {
                    provider,
                    representation: Representation::Data {
                        format,
                        kind: kind.clone(),
                    },
                });
            }
        }
    }
    // UIImage copies can advertise only com.apple.uikit.image, not public.png.
    // Ask the supported object-loading API after all encoded representations.
    for provider in providers.iter() {
        // SAFETY: Foundation's generated bindings omit this generic class API;
        // its documented argument is UIImage's NSItemProviderReading class.
        let can_load: bool =
            unsafe { msg_send![&*provider, canLoadObjectOfClass: UIImage::class()] };
        if can_load {
            candidates.push(Candidate {
                provider,
                representation: Representation::Object(ObjectKind::Image),
            });
        }
    }
    Ok(Read {
        count: candidates.len(),
        candidates: Arc::new(MainThreadBound::new(candidates, main)),
        deadline,
    })
}

/// UIImage is immutable and has generated Send/Sync. Encode on the worker,
/// never in a provider callback or on the UI queue. Reuse the decoder's cap.
fn encode_image(image: &UIImage) -> Result<Option<Vec<u8>>> {
    // SAFETY: read-only properties of the retained immutable image; no view,
    // drawing context, or main-thread-only UIKit object is accessed here.
    let (size, scale, animated) = unsafe { (image.size(), image.scale(), image.images()) };
    let pixels = (size.width * scale).ceil() * (size.height * scale).ceil();
    if !pixels.is_finite() || pixels <= 0. || animated.is_some_and(|frames| frames.len() > 1) {
        return Err(Error::ImageObject.into());
    }
    if pixels > super::IMAGE_PIXEL_LIMIT as f64 {
        return Err(Error::ImageDimensions {
            limit: super::IMAGE_PIXEL_LIMIT,
        }
        .into());
    }
    let Some(data) = image.png_representation() else {
        return Ok(None);
    };
    if data.len() > super::IMAGE_LIMIT {
        return Err(Error::Size {
            limit: super::IMAGE_LIMIT,
        }
        .into());
    }
    Ok(Some(data.to_vec()))
}
