//! UIKit acquisition must stay on the main thread, before worker preparation.

use gpui::{ClipboardItem, ImageFormat};
use objc2::MainThreadMarker;
use objc2_foundation::{NSArray, NSString};
use objc2_ui_kit::UIPasteboard;

use super::{Error, read_with};
use crate::Result;

pub(crate) fn read(image_only: bool) -> Result<Option<ClipboardItem>> {
    let main = MainThreadMarker::new().ok_or(Error::Unsupported)?;
    read_pasteboard(&UIPasteboard::generalPasteboard(), image_only, main)
}

pub(super) fn read_pasteboard(
    pasteboard: &UIPasteboard,
    image_only: bool,
    _main: MainThreadMarker,
) -> Result<Option<ClipboardItem>> {
    // GPUI's iOS reader returns only text for mixed copies. Read the requested
    // representations directly so image-only paste never loses the image.
    read_with(image_only, |format, limit| {
        let kind = NSString::from_str(match format {
            None => "public.utf8-plain-text",
            Some(ImageFormat::Png) => "public.png",
            Some(ImageFormat::Jpeg) => "public.jpeg",
            Some(ImageFormat::Gif) => "com.compuserve.gif",
            Some(ImageFormat::Webp) => "org.webmproject.webp",
            Some(ImageFormat::Bmp) => "com.microsoft.bmp",
            Some(ImageFormat::Tiff) => "public.tiff",
            Some(_) => return Ok(None),
        });
        if !pasteboard.containsPasteboardTypes(&NSArray::from_slice(&[&*kind])) {
            return Ok(None);
        }
        let Some(data) = pasteboard.dataForPasteboardType(&kind) else {
            return Ok(None);
        };
        // UIKit materializes NSData first; bound the Rust copy before hashing
        // or decoding. All property access stays on this main-thread call.
        if data.len() > limit {
            return Err(Error::Size { limit }.into());
        }
        Ok(Some(data.to_vec()))
    })
}
