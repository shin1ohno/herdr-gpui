#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use gpui::{ClipboardEntry, ImageFormat};
use objc2::MainThreadMarker;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSArray, NSData, NSDictionary, NSString};
use objc2_ui_kit::UIPasteboard;

/// A real UIKit pasteboard with both representations on the same item,
/// rather than a GPUI item that its platform reader can never return.
#[allow(
    unsafe_code,
    reason = "private pasteboard is accessed only on the main thread"
)]
pub(crate) fn mixed_uikit_copy_preserves_normal_text_and_image_only_image() {
    let main = MainThreadMarker::new().expect("run on the simulator's main thread");
    let pasteboard = UIPasteboard::pasteboardWithUniqueName();
    let text_type = NSString::from_str("public.utf8-plain-text");
    let image_type = NSString::from_str("public.png");
    let text = NSString::from_str("image caption");
    let png = NSData::with_bytes(b"\x89PNG\r\n\x1a\n");
    let item = NSDictionary::from_slices(
        &[&*text_type, &*image_type],
        &[&*text as &AnyObject, &*png as &AnyObject],
    );
    // setData alone cannot express NSString and NSData on one item.
    // SAFETY: unique board, main-thread marker above, no concurrent access.
    unsafe { pasteboard.setItems(&NSArray::from_slice(&[&*item])) };
    assert_eq!(
        read_pasteboard(&pasteboard, false, main)
            .unwrap()
            .unwrap()
            .text(),
        Some("image caption".into())
    );
    let image = read_pasteboard(&pasteboard, true, main).unwrap().unwrap();
    assert!(matches!(image.entries(), [ClipboardEntry::Image(image)]
        if image.format == ImageFormat::Png && image.bytes.as_ref() == png.to_vec()));
}

pub(crate) fn empty_uikit_copy_and_oversized_native_data_are_bounded() {
    let main = MainThreadMarker::new().expect("run on the simulator's main thread");
    let pasteboard = UIPasteboard::pasteboardWithUniqueName();
    assert!(read_pasteboard(&pasteboard, false, main).unwrap().is_none());
    assert!(read_pasteboard(&pasteboard, true, main).unwrap().is_none());
    let png = NSData::with_bytes(&[0; IMAGE_LIMIT + 1]);
    pasteboard.setData_forPasteboardType(&png, &NSString::from_str("public.png"));
    let error = read_pasteboard(&pasteboard, true, main).unwrap_err();
    assert!(matches!(error, TestError::Clipboard(Error::Size { limit }) if limit == IMAGE_LIMIT));
}
