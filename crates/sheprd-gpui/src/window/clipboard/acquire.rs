//! Representation selection and bounds shared by native clipboard readers.

use super::{Error, IMAGE_LIMIT, TEXT_LIMIT};
use crate::Result;
use gpui::{ClipboardItem, Image, ImageFormat};

pub(super) fn read_with(
    image_only: bool,
    mut acquire: impl FnMut(Option<ImageFormat>, usize) -> Result<Option<Vec<u8>>>,
) -> Result<Option<ClipboardItem>> {
    if !image_only && let Some(bytes) = acquire(None, TEXT_LIMIT)? {
        if bytes.len() > TEXT_LIMIT {
            return Err(Error::Size { limit: TEXT_LIMIT }.into());
        }
        let text = String::from_utf8(bytes).map_err(|error| Error::Encoding(error.utf8_error()))?;
        return Ok(Some(ClipboardItem::new_string(text)));
    }
    // TIFF is last: AppKit apps often publish it beside a PNG that needs no
    // conversion, while Preview and some browsers publish only TIFF.
    for format in [
        ImageFormat::Png,
        ImageFormat::Jpeg,
        ImageFormat::Gif,
        ImageFormat::Webp,
        ImageFormat::Bmp,
        ImageFormat::Tiff,
    ] {
        if let Some(bytes) = acquire(Some(format), IMAGE_LIMIT)? {
            if bytes.len() > IMAGE_LIMIT {
                return Err(Error::Size { limit: IMAGE_LIMIT }.into());
            }
            if !bytes.is_empty() {
                // From<Image> moves the bytes; new_image(&Image) would clone them.
                return Ok(Some(Image::from_bytes(format, bytes).into()));
            }
        }
    }
    Ok(None)
}
