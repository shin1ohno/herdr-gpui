//! Exercise the production reader with native providers, not a second resolver.
#![allow(
    unsafe_code,
    reason = "main-thread private boards and sendable native fixture callbacks"
)]

use super::*;
use block2::{DynBlock, RcBlock};
use objc2::rc::Retained;
use objc2_foundation::{
    NSError, NSItemProvider, NSItemProviderRepresentationVisibility, NSProgress,
};
use std::{
    ptr::{self, NonNull},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Default)]
struct State {
    calls: AtomicUsize,
    completed: AtomicBool,
    progress: Mutex<Option<Retained<NSProgress>>>,
    release: Mutex<bool>,
    wake: Condvar,
}
impl State {
    fn release(&self) {
        *self.release.lock().unwrap_or_else(|e| e.into_inner()) = true;
        self.wake.notify_all();
    }
}

enum Response {
    Data(Vec<u8>),
    Delayed(Vec<u8>),
    Denied,
    Never,
}

// Foundation explicitly permits this provider completion on any queue.
// block2 cannot express that native contract; own a copied block and expose
// only invocation, so no borrowed callback or arbitrary block crosses threads.
struct NativeCompletion(RcBlock<dyn Fn(*mut NSData, *mut NSError)>);
// SAFETY: this is exclusively the copied NSItemProvider loading completion,
// whose documented lifetime and any-queue invocation contract is thread-safe.
unsafe impl Send for NativeCompletion {}
impl NativeCompletion {
    fn call(&self, data: *mut NSData, error: *mut NSError) {
        self.0.call((data, error));
    }
}

fn register(provider: &NSItemProvider, kind: &str, response: Response) -> Arc<State> {
    let state = Arc::new(State::default());
    let loaded = state.clone();
    let handler = RcBlock::new(
        move |reply: NonNull<DynBlock<dyn Fn(*mut NSData, *mut NSError)>>| {
            loaded.calls.fetch_add(1, Ordering::Release);
            let progress = NSProgress::progressWithTotalUnitCount(1);
            let pointer = Retained::as_ptr(&progress).cast_mut();
            *loaded.progress.lock().unwrap_or_else(|e| e.into_inner()) = Some(progress);
            let mut data = None;
            let mut error = None;
            match &response {
                Response::Never => return pointer,
                Response::Data(bytes) => data = Some(NSData::with_bytes(bytes)),
                Response::Denied => {
                    error = Some(NSError::new(
                        3072,
                        &NSString::from_str("NSCocoaErrorDomain"),
                    ))
                }
                Response::Delayed(bytes) => {
                    // Foundation may invoke a publisher on main. Return its
                    // progress immediately; only the fixture worker waits.
                    // SAFETY: Foundation's completion is callable from any
                    // thread and remains valid after copying the native block.
                    let reply = NativeCompletion(unsafe { RcBlock::copy(reply.as_ptr()) }.unwrap());
                    let state = loaded.clone();
                    let bytes = bytes.clone();
                    std::thread::spawn(move || {
                        let release = state.release.lock().unwrap_or_else(|e| e.into_inner());
                        let (release, _) = state
                            .wake
                            .wait_timeout_while(release, Duration::from_secs(5), |open| !*open)
                            .unwrap_or_else(|e| e.into_inner());
                        let data = (*release).then(|| NSData::with_bytes(&bytes));
                        let error = (!*release)
                            .then(|| NSError::new(99, &NSString::from_str("HerdrFixtureDeadline")));
                        reply.call(
                            data.as_ref()
                                .map_or(ptr::null_mut(), |d| Retained::as_ptr(d).cast_mut()),
                            error
                                .as_ref()
                                .map_or(ptr::null_mut(), |e| Retained::as_ptr(e).cast_mut()),
                        );
                        state.completed.store(true, Ordering::Release);
                    });
                    return pointer;
                }
            }
            // SAFETY: reply is live during this handler; Foundation values remain
            // owned through the call. The callback may copy them, never outlive them.
            unsafe { reply.as_ref() }.call((
                data.as_ref()
                    .map_or(ptr::null_mut(), |v| Retained::as_ptr(v).cast_mut()),
                error
                    .as_ref()
                    .map_or(ptr::null_mut(), |v| Retained::as_ptr(v).cast_mut()),
            ));
            loaded.completed.store(true, Ordering::Release);
            pointer
        },
    );
    // SAFETY: captures are synchronized Rust state and immutable byte vectors.
    unsafe {
        provider.registerDataRepresentationForTypeIdentifier_visibility_loadHandler(
            &NSString::from_str(kind),
            NSItemProviderRepresentationVisibility::All,
            &handler,
        );
    }
    state
}

fn native_providers(providers: &[&NSItemProvider]) -> Retained<NSArray<NSItemProvider>> {
    NSArray::from_slice(providers)
}

fn capture(
    providers: &NSArray<NSItemProvider>,
    image_only: bool,
    main: MainThreadMarker,
) -> Result<Read> {
    read_providers_until(
        providers,
        image_only,
        main,
        Instant::now() + Duration::from_secs(30),
    )
}

fn load_providers(
    providers: &NSArray<NSItemProvider>,
    image_only: bool,
    main: MainThreadMarker,
) -> Result<Option<gpui::ClipboardItem>> {
    run_read(capture(providers, image_only, main)?, || false, || {})
}

pub(crate) fn fallback_and_empty_text() {
    let main = MainThreadMarker::new().unwrap();
    for nil in [true, false] {
        let (first, png_calls) = if nil {
            nil_data::nil_png_provider(main)
        } else {
            let provider = NSItemProvider::new();
            let calls = register(&provider, "public.png", Response::Data(Vec::new()));
            (provider, calls)
        };
        let jpeg_calls = register(
            &first,
            "public.jpeg",
            Response::Data(vec![0xff, 0xd8, 0xff]),
        );
        let board = native_providers(&[&first]);
        let item = load_providers(&board, true, main).unwrap().unwrap();
        assert!(
            matches!(item.entries(), [ClipboardEntry::Image(image)] if image.format == ImageFormat::Jpeg)
        );
        assert_eq!(png_calls.calls.load(Ordering::Acquire), 1);
        assert_eq!(jpeg_calls.calls.load(Ordering::Acquire), 1);
    }
    let (first, nil) = nil_data::nil_png_provider(main);
    let second = NSItemProvider::new();
    let png = register(
        &second,
        "public.png",
        Response::Data(b"\x89PNG\r\n\x1a\n".to_vec()),
    );
    let board = native_providers(&[&first, &second]);
    assert!(load_providers(&board, true, main).unwrap().is_some());
    assert_eq!(nil.calls.load(Ordering::Acquire), 1);
    assert_eq!(png.calls.load(Ordering::Acquire), 1);

    let first = NSItemProvider::new();
    let text = register(&first, "public.utf8-plain-text", Response::Data(Vec::new()));
    let image = register(
        &first,
        "public.png",
        Response::Data(b"\x89PNG\r\n\x1a\n".to_vec()),
    );
    let board = native_providers(&[&first]);
    let empty_text = load_providers(&board, false, main).unwrap().unwrap();
    // GPUI's convenience text() returns None for an empty string. The actual
    // clipboard entry must still be text, with no image acquisition.
    assert!(
        matches!(empty_text.entries(), [ClipboardEntry::String(text)] if text.text().is_empty())
    );
    assert_eq!(text.calls.load(Ordering::Acquire), 1);
    assert_eq!(image.calls.load(Ordering::Acquire), 0);
    assert!(load_providers(&board, true, main).unwrap().is_some());
    assert_eq!(text.calls.load(Ordering::Acquire), 1);
    assert_eq!(image.calls.load(Ordering::Acquire), 1);
}

pub(crate) fn delayed_provider_leaves_main_run_loop_live() {
    let main = MainThreadMarker::new().unwrap();
    let provider = NSItemProvider::new();
    let state = register(
        &provider,
        "public.png",
        Response::Delayed(b"\x89PNG\r\n\x1a\n".to_vec()),
    );
    let board = native_providers(&[&provider]);
    let read = capture(&board, true, main).unwrap();
    assert_eq!(
        state.calls.load(Ordering::Acquire),
        0,
        "metadata capture must not load"
    );
    let mut serviced_while_loading = false;
    let item = run_read(
        read,
        || false,
        || {
            if state.calls.load(Ordering::Acquire) > 0 {
                serviced_while_loading = true;
                state.release();
            }
        },
    )
    .unwrap();
    assert!(serviced_while_loading && item.is_some());
}

pub(crate) fn denial_stops_fallback_and_keeps_native_source() {
    use std::error::Error as _;
    let main = MainThreadMarker::new().unwrap();
    let provider = NSItemProvider::new();
    let denied = register(&provider, "public.png", Response::Denied);
    let jpeg = register(
        &provider,
        "public.jpeg",
        Response::Data(vec![0xff, 0xd8, 0xff]),
    );
    let error = load_providers(&native_providers(&[&provider]), true, main).unwrap_err();
    assert!(error.source().is_some());
    assert!(matches!(error, TestError::Clipboard(Error::Provider(_))));
    assert_eq!(denied.calls.load(Ordering::Acquire), 1);
    assert_eq!(
        jpeg.calls.load(Ordering::Acquire),
        0,
        "a denial is not a fallback request"
    );
}

pub(crate) fn cancellation_and_one_deadline_cancel_native_progress() {
    let main = MainThreadMarker::new().unwrap();
    for cancel in [true, false] {
        let provider = NSItemProvider::new();
        let state = register(&provider, "public.png", Response::Never);
        let jpeg = register(
            &provider,
            "public.jpeg",
            Response::Data(vec![0xff, 0xd8, 0xff]),
        );
        let board = native_providers(&[&provider]);
        let read =
            read_providers_until(&board, true, main, Instant::now() + Duration::from_secs(1))
                .unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let observe = cancelled.clone();
        let result = run_read(
            read,
            move || observe.load(Ordering::Acquire),
            || {
                if cancel && state.calls.load(Ordering::Acquire) > 0 {
                    cancelled.store(true, Ordering::Release);
                }
            },
        );
        if cancel {
            assert!(matches!(
                result,
                Err(TestError::Clipboard(Error::Cancelled))
            ));
        } else {
            assert!(matches!(result, Err(TestError::Clipboard(Error::Timeout))));
        }
        assert_eq!(jpeg.calls.load(Ordering::Acquire), 0);
        // The production guard cancels the NSItemProvider load's progress.
        // Whether Foundation propagates that to this child progress is checked
        // rather than assuming a successful cancel call proves native state.
        let end = Instant::now() + Duration::from_secs(2);
        loop {
            if state
                .progress
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_ref()
                .is_some_and(|p| p.isCancelled())
            {
                break;
            }
            assert!(
                Instant::now() < end,
                "native provider did not observe cancellation"
            );
            pump();
        }
    }
}

pub(crate) fn provider_limit_rejects_before_loading() {
    let main = MainThreadMarker::new().unwrap();
    let providers: Vec<_> = (0..=MAX_PROVIDERS).map(|_| NSItemProvider::new()).collect();
    let states: Vec<_> = providers
        .iter()
        .map(|p| register(p, "public.png", Response::Never))
        .collect();
    let refs: Vec<_> = providers.iter().map(|p| &**p).collect();
    let result = capture(&native_providers(&refs), true, main);
    assert!(matches!(
        result,
        Err(TestError::Clipboard(Error::Providers { .. }))
    ));
    assert!(states.iter().all(|s| s.calls.load(Ordering::Acquire) == 0));
}

pub(crate) fn late_cancelled_callback_cannot_replace_a_later_read() {
    let main = MainThreadMarker::new().unwrap();
    let old = NSItemProvider::new();
    let state = register(&old, "public.png", Response::Delayed(b"old image".to_vec()));
    let old_board = native_providers(&[&old]);
    let read = capture(&old_board, true, main).unwrap();
    let flag = Arc::new(AtomicBool::new(false));
    let observe = flag.clone();
    let result = run_read(
        read,
        move || observe.load(Ordering::Acquire),
        || {
            if state.calls.load(Ordering::Acquire) > 0 {
                flag.store(true, Ordering::Release);
            }
        },
    );
    assert!(matches!(
        result,
        Err(TestError::Clipboard(Error::Cancelled))
    ));
    state.release();
    let next = NSItemProvider::new();
    register(&next, "public.png", Response::Data(b"new image".to_vec()));
    let item = load_providers(&native_providers(&[&next]), true, main)
        .unwrap()
        .unwrap();
    assert!(
        matches!(item.entries(), [ClipboardEntry::Image(image)] if image.bytes.as_slice() == b"new image")
    );
    let end = Instant::now() + Duration::from_secs(2);
    while !state.completed.load(Ordering::Acquire) {
        assert!(
            Instant::now() < end,
            "cancelled provider never delivered its late callback"
        );
        pump();
    }
}

pub(crate) fn uiimage_and_encoded_png_are_loaded_through_the_native_board() {
    use objc2::{AnyThread, runtime::ProtocolObject};
    use objc2_ui_kit::UIImage;
    let main = MainThreadMarker::new().unwrap();
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
        1,
        1,
        image::Rgba([0, 0, 255, 255]),
    ))
    .write_to(&mut encoded, image::ImageFormat::Png)
    .unwrap();
    let bytes = encoded.into_inner();
    let image = UIImage::imageWithData(&NSData::with_bytes(&bytes)).unwrap();
    let provider =
        NSItemProvider::initWithObject(NSItemProvider::alloc(), ProtocolObject::from_ref(&*image));
    let native_board = native_providers(&[&provider]);
    let item = load_providers(&native_board, true, main).unwrap().unwrap();
    let decoded = image::load_from_memory(match item.entries() {
        [ClipboardEntry::Image(image)] => image.bytes.as_slice(),
        _ => panic!("native UIImage did not become image bytes"),
    })
    .unwrap();
    assert_eq!((decoded.width(), decoded.height()), (1, 1));
    let uiimage_board = empty_board();
    // SAFETY: unique board, main-thread-only access, immutable UIImage.
    unsafe {
        uiimage_board.setImage(Some(&image));
    }
    // UIKit re-encodes UIImage; assert decoded content rather than its encoding.
    finish_publication(&uiimage_board, &NSString::from_str("public.png"), None);
    let uiimage = load(&uiimage_board, true, main).unwrap().unwrap();
    let rgba = match uiimage.entries() {
        [ClipboardEntry::Image(image)] => image::load_from_memory(&image.bytes).unwrap().to_rgba8(),
        _ => panic!("UIImage board did not return an image"),
    };
    assert_eq!(rgba.dimensions(), (1, 1));
    assert_eq!(rgba.get_pixel(0, 0).0, [0, 0, 255, 255]);
    let raw_board = empty_board();
    raw_board.setData_forPasteboardType(
        &NSData::with_bytes(&bytes),
        &NSString::from_str("public.png"),
    );
    finish_publication(
        &raw_board,
        &NSString::from_str("public.png"),
        Some(bytes.len()),
    );
    let item = load(&raw_board, true, main).unwrap().unwrap();
    assert!(
        matches!(item.entries(), [ClipboardEntry::Image(image)] if image.bytes.as_slice() == bytes)
    );
}

/// Last: deliberately noncompleting providers test the native callback ceiling.
/// The executable exits afterwards; no user's process or board is involved.
pub(crate) fn ignored_cancellation_cannot_accumulate_native_requests() {
    let main = MainThreadMarker::new().unwrap();
    let provider = NSItemProvider::new();
    let state = register(&provider, "public.png", Response::Never);
    let board = native_providers(&[&provider]);
    let cancel = Arc::new(AtomicBool::new(false));
    let (send, results) = std::sync::mpsc::channel();
    let mut workers = Vec::new();
    for _ in 0..=MAX_ACQUISITIONS {
        let read = capture(&board, true, main).unwrap();
        let observe = cancel.clone();
        let send = send.clone();
        workers.push(std::thread::spawn(move || {
            let _ = send.send(read.wait(|| observe.load(Ordering::Acquire)));
        }));
    }
    drop(send);
    let mut observed = Vec::new();
    let end = Instant::now() + Duration::from_secs(5);
    loop {
        while let Ok(result) = results.try_recv() {
            observed.push(result);
        }
        let busy = observed
            .iter()
            .any(|result| matches!(result, Err(TestError::Clipboard(Error::ProviderBusy))));
        if busy && state.calls.load(Ordering::Acquire) > 0 {
            break;
        }
        assert!(
            Instant::now() < end,
            "native request ceiling did not refuse excess work"
        );
        pump();
    }
    cancel.store(true, Ordering::Release);
    while observed.len() < workers.len() {
        while let Ok(result) = results.try_recv() {
            observed.push(result);
        }
        assert!(Instant::now() < end, "cancelled native read did not finish");
        pump();
    }
    for worker in workers {
        worker.join().unwrap();
    }
    assert!(state.calls.load(Ordering::Acquire) <= MAX_ACQUISITIONS);
    assert!(observed.iter().all(|result| matches!(
        result,
        Err(TestError::Clipboard(Error::ProviderBusy | Error::Cancelled))
    )));
}

#[path = "providers/object_fallback.rs"]
mod object_fallback;
pub(crate) use object_fallback::nil_text_object_uses_same_provider_data;

#[path = "providers/nil_data.rs"]
mod nil_data;
