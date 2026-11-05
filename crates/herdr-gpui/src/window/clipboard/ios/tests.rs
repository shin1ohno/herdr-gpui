#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;

#[path = "tests/providers.rs"]
mod providers;
use gpui::{ClipboardEntry, ImageFormat};
use objc2::MainThreadMarker;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSArray, NSData, NSDictionary, NSString};
use objc2_ui_kit::UIPasteboard;
pub(crate) use providers::{
    cancellation_and_one_deadline_cancel_native_progress,
    delayed_provider_leaves_main_run_loop_live, denial_stops_fallback_and_keeps_native_source,
    fallback_and_empty_text, ignored_cancellation_cannot_accumulate_native_requests,
    late_cancelled_callback_cannot_replace_a_later_read, nil_text_object_uses_same_provider_data,
    provider_limit_rejects_before_loading,
    uiimage_and_encoded_png_are_loaded_through_the_native_board,
};

/// A real UIKit pasteboard with both representations on the same item,
/// rather than a GPUI item that its platform reader can never return.
#[allow(
    unsafe_code,
    reason = "private pasteboard is accessed only on the main thread"
)]
pub(crate) fn mixed_uikit_copy_preserves_normal_text_and_image_only_image() {
    let main = MainThreadMarker::new().expect("run on the simulator's main thread");
    let pasteboard = empty_board();
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
    let diagnostic = std::env::var("HERDR_TEST_CLIPBOARD_DIAGNOSTIC").unwrap_or_default();
    if diagnostic == "sync" {
        // SAFETY: bounded fixed fixture, on the UIKit main thread; diagnosis only.
        let text = unsafe { pasteboard.string() };
        eprintln!(
            "native mixed board string={text:?} text-data={:?}",
            pasteboard
                .dataForPasteboardType(&text_type)
                .map(|data| data.to_vec())
        );
    }
    // The fixture is also this process's publisher. Verify its asynchronous
    // publication separately, before measuring the production acquisition.
    // Explicit diagnostic modes retain the original unprepared comparison.
    if diagnostic.is_empty() || diagnostic == "direct" {
        finish_publication(&pasteboard, &text_type, Some(13));
    }
    assert_eq!(
        load(&pasteboard, false, main).unwrap().unwrap().text(),
        Some("image caption".into())
    );
    let image = load(&pasteboard, true, main).unwrap().unwrap();
    assert!(matches!(image.entries(), [ClipboardEntry::Image(image)]
        if image.format == ImageFormat::Png && image.bytes.as_slice() == png.to_vec()));
}

pub(crate) fn empty_uikit_copy_and_oversized_native_data_are_bounded() {
    let main = MainThreadMarker::new().expect("run on the simulator's main thread");
    let pasteboard = empty_board();
    assert!(load(&pasteboard, false, main).unwrap().is_none());
    assert!(load(&pasteboard, true, main).unwrap().is_none());
    let png = NSData::with_bytes(&[0; IMAGE_LIMIT + 1]);
    pasteboard.setData_forPasteboardType(&png, &NSString::from_str("public.png"));
    finish_publication(
        &pasteboard,
        &NSString::from_str("public.png"),
        Some(IMAGE_LIMIT + 1),
    );
    let error = load(&pasteboard, true, main).unwrap_err();
    assert!(matches!(error, TestError::Clipboard(Error::Size { limit }) if limit == IMAGE_LIMIT));
}

/// Native acquisition failures keep the provider's typed NSError as a source.
#[allow(
    unsafe_code,
    reason = "sendable provider callback with valid borrowed block arguments"
)]
pub(crate) fn provider_failure_preserves_the_native_source() {
    use block2::{DynBlock, RcBlock};
    use objc2::rc::Retained;
    use objc2_foundation::{
        NSError, NSItemProvider, NSItemProviderRepresentationVisibility, NSProgress,
    };
    use std::{
        error::Error as _,
        ptr::{self, NonNull},
    };

    let main = MainThreadMarker::new().expect("run on the simulator's main thread");
    let provider = NSItemProvider::new();
    let kind = NSString::from_str("public.png");
    let handler = RcBlock::new(
        |completion: NonNull<DynBlock<dyn Fn(*mut NSData, *mut NSError)>>| {
            let error = NSError::new(17, &NSString::from_str("HerdrClipboardRegression"));
            // SAFETY: NSItemProvider supplies a live callback for this call; neither
            // its pointer nor the temporary NSError escapes this invocation.
            unsafe { completion.as_ref() }
                .call((ptr::null_mut(), Retained::as_ptr(&error).cast_mut()));
            ptr::null_mut::<NSProgress>()
        },
    );
    // SAFETY: this callback captures no state, allocates immutable Foundation
    // values on the invoking thread, and invokes its completion once.
    unsafe {
        provider.registerDataRepresentationForTypeIdentifier_visibility_loadHandler(
            &kind,
            NSItemProviderRepresentationVisibility::All,
            &handler,
        );
    }
    let providers = NSArray::from_slice(&[&*provider]);
    let read = read_providers_until(
        &providers,
        true,
        main,
        std::time::Instant::now() + std::time::Duration::from_secs(30),
    )
    .unwrap();
    let error = run_read(read, || false, || {}).unwrap_err();
    assert!(error.source().is_some());
    let TestError::Clipboard(Error::Provider(source)) = error else {
        panic!("expected typed provider error")
    };
    // Foundation's acquisition API wraps the publisher error; retain both
    // that boundary error and its original underlying NSError unchanged.
    assert_eq!(source.code(), -1000);
    assert_eq!(source.domain().to_string(), "NSItemProviderErrorDomain");
    let underlying = source
        .userInfo()
        .objectForKey(&NSString::from_str("NSUnderlyingError"))
        .unwrap();
    let underlying = underlying.downcast_ref::<NSError>().unwrap();
    assert_eq!(underlying.code(), 17);
    assert_eq!(underlying.domain().to_string(), "HerdrClipboardRegression");
}

/// The reader returns before loading bytes. Run the real main loop while the
/// ordered worker waits, as the app does; never block UIKit waiting for itself.
fn load(
    pasteboard: &UIPasteboard,
    image_only: bool,
    main: MainThreadMarker,
) -> Result<Option<gpui::ClipboardItem>> {
    run_read(
        read_pasteboard(pasteboard, image_only, main)?,
        || false,
        || {},
    )
}

fn pump() {
    use objc2_foundation::{NSDate, NSRunLoop};
    NSRunLoop::mainRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.01));
}

fn run_read(
    read: Read,
    cancelled: impl Fn() -> bool + Send + 'static,
    mut tick: impl FnMut(),
) -> Result<Option<gpui::ClipboardItem>> {
    use std::{
        sync::mpsc,
        time::{Duration, Instant},
    };
    let (send, reply) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let _ = send.send(read.wait(cancelled));
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        tick();
        match reply.try_recv() {
            Ok(result) => {
                worker.join().expect("clipboard worker panicked");
                if let Err(TestError::Clipboard(Error::Provider(source))) = &result
                    && let Some(underlying) = source
                        .userInfo()
                        .objectForKey(&NSString::from_str("NSUnderlyingError"))
                    && let Some(error) = underlying.downcast_ref::<objc2_foundation::NSError>()
                {
                    eprintln!("native clipboard underlying error: {error:?}");
                }
                return result;
            }
            Err(mpsc::TryRecvError::Disconnected) => panic!("clipboard worker disconnected"),
            Err(mpsc::TryRecvError::Empty) => {}
        }
        assert!(
            Instant::now() < deadline,
            "clipboard provider did not finish"
        );
        pump();
    }
}

/// itemProviders on named pasteboards advertises metadata but fails native
/// payload loading (PBErrorDomain) on the tested iPadOS runtime. These checks
/// run only in the dedicated foreground test app on the explicitly selected
/// simulator, and use its general board with fixed fixture content.
#[allow(unsafe_code, reason = "main-thread-only dedicated test pasteboard")]
fn empty_board() -> objc2::rc::Retained<UIPasteboard> {
    let _main = MainThreadMarker::new().expect("native tests require main thread");
    let board = UIPasteboard::generalPasteboard();
    board.setItemProviders_localOnly_expirationDate(&NSArray::new(), true, None);
    board
}

/// Publishing a board can itself invoke providers before a reader exists.
/// These roundtrip fixtures establish publication through a real native data
/// completion first. Payload acquisition counters, error/cancel/liveness tests
/// use direct native providers and never include this separate setup phase.
/// No synchronous pasteboard payload getter or guessed delay is used here.
#[allow(unsafe_code, reason = "native fixture publication completion on main")]
fn finish_publication(board: &UIPasteboard, kind: &NSString, expected_size: Option<usize>) {
    let providers = unsafe { board.itemProviders() };
    let provider = providers.firstObject().expect("published fixture provider");
    let (send, reply) = std::sync::mpsc::sync_channel(1);
    let callback = block2::RcBlock::new(
        move |data: *mut NSData, error: *mut objc2_foundation::NSError| {
            // SAFETY: native completion arguments are valid for this call. Only
            // size and the retained immutable error cross the callback boundary.
            let result = unsafe {
                if let Some(error) = objc2::rc::Retained::retain(error) {
                    Err(error)
                } else {
                    Ok(data.as_ref().map(NSData::len))
                }
            };
            let _ = send.try_send(result);
        },
    );
    let _progress = unsafe {
        provider.loadDataRepresentationForTypeIdentifier_completionHandler(kind, &callback)
    };
    let end = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        if let Ok(result) = reply.try_recv() {
            let size = result.unwrap().expect("published fixture data");
            if let Some(expected_size) = expected_size {
                assert_eq!(size, expected_size, "fixture publication size");
            } else {
                assert!(size > 0, "published UIImage data");
            }
            eprintln!("native fixture publication completed: {kind:?}, {size} bytes");
            break;
        }
        assert!(
            std::time::Instant::now() < end,
            "fixture publication did not complete"
        );
        pump();
    }
}
