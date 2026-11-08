//! One native request, with race-safe cancellation and no callback waits.

#![allow(
    unsafe_code,
    reason = "sendable NSItemProvider completion with valid borrowed arguments" // i18n: data
)]

use super::{Candidate, Error, ObjectKind, Representation, Result};
use block2::RcBlock;
use dispatch2::{DispatchQueue, MainThreadBound};
use gpui::ImageFormat;
use objc2::{
    ClassType, MainThreadMarker, msg_send,
    rc::Retained,
    runtime::{AnyObject, ProtocolObject},
};
use objc2_foundation::{NSData, NSError, NSItemProviderReading, NSProgress, NSString};
use objc2_ui_kit::UIImage;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{RecvTimeoutError, sync_channel},
    },
    time::{Duration, Instant},
};

pub(super) enum Payload {
    Data(Vec<u8>),
    Image(Retained<UIImage>),
}

pub(super) struct Loaded {
    pub(super) payload: Option<Payload>,
    pub(super) format: Option<ImageFormat>,
}

// Cancellation can finish the FIFO worker before a provider acknowledges it.
// Keep this separate lease until native completion or callback destruction, so
// providers that ignore cancellation cannot accumulate across new pastes.
static NATIVE_REQUESTS: AtomicUsize = AtomicUsize::new(0);
struct NativeLease(AtomicBool);
impl NativeLease {
    fn acquire() -> Result<Self> {
        NATIVE_REQUESTS
            .try_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < super::super::MAX_ACQUISITIONS).then_some(count + 1)
            })
            .map_err(|_| Error::ProviderBusy)?;
        Ok(Self(AtomicBool::new(false)))
    }
    fn release(&self) {
        if !self.0.swap(true, Ordering::AcqRel) {
            NATIVE_REQUESTS.fetch_sub(1, Ordering::AcqRel);
        }
    }
}
impl Drop for NativeLease {
    fn drop(&mut self) {
        self.release();
    }
}

#[derive(Default)]
struct Attempt {
    done: AtomicBool,
    progress: Mutex<Option<Retained<NSProgress>>>,
}

impl Attempt {
    fn install(&self, progress: Retained<NSProgress>) {
        let mut slot = self.progress.lock().unwrap_or_else(|e| e.into_inner());
        if self.done.load(Ordering::Acquire) {
            progress.cancel();
        } else {
            *slot = Some(progress);
        }
    }

    fn cancel(&self) {
        self.done.store(true, Ordering::Release);
        let progress = self
            .progress
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        if let Some(progress) = progress {
            progress.cancel();
        }
    }
}

struct Guard(Arc<Attempt>);
impl Drop for Guard {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

pub(super) fn load(
    candidates: Arc<MainThreadBound<Vec<Candidate>>>,
    index: usize,
    deadline: Instant,
    cancelled: &impl Fn() -> bool,
) -> Result<Loaded> {
    let attempt = Arc::new(Attempt::default());
    let _guard = Guard(attempt.clone());
    let (send, reply) = sync_channel(1);
    DispatchQueue::main().exec_async(move || {
        if attempt.done.load(Ordering::Acquire) || Instant::now() >= deadline {
            return;
        }
        let Some(main) = MainThreadMarker::new() else {
            let _ = send.try_send(Err(Error::Unsupported.into()));
            return;
        };
        let lease = match NativeLease::acquire() {
            Ok(lease) => lease,
            Err(error) => { let _ = send.try_send(Err(error)); return; }
        };
        let candidate = &candidates.get(main)[index];
        if let Representation::Object(kind) = &candidate.representation {
            let kind = *kind;
            let class = match kind {
                ObjectKind::Image => UIImage::class(),
                ObjectKind::Text => NSString::class(),
            };
            let loading = attempt.clone();
            let completion = RcBlock::new(
                move |object: *mut ProtocolObject<dyn NSItemProviderReading>, error: *mut NSError| {
                    lease.release();
                    if loading.done.swap(true, Ordering::AcqRel) { return; }
                    // SAFETY: callback arguments are live through this call.
                    let result = unsafe { object_result(object, error, kind) };
                    let _ = send.try_send(result);
                },
            );
            // SAFETY: documented Foundation object-loading selector returns
            // NSProgress. Generated bindings omit the generic class method.
            // UIImage implements NSItemProviderReading; callback is sendable.
            let progress: Retained<NSProgress> = unsafe {
                msg_send![&*candidate.provider, loadObjectOfClass: class, completionHandler: &*completion]
            };
            attempt.install(progress);
            return;
        }
        let Representation::Data { format, kind } = &candidate.representation else { return; };
        let format = *format;
        let limit = if format.is_some() {
            super::super::IMAGE_LIMIT
        } else {
            super::super::TEXT_LIMIT
        };
        let completion_attempt = attempt.clone();
        let completion = RcBlock::new(move |data: *mut NSData, error: *mut NSError| {
            // Claims completion once, before copying. Cancellation and duplicate
            // callbacks cannot add work; a racing copy is still byte-bounded.
            lease.release();
            if completion_attempt.done.swap(true, Ordering::AcqRel) {
                return;
            }
            // SAFETY: arguments remain valid during this call. NSError is
            // retained and thread-safe; immutable NSData is copied after bounds.
            let result = unsafe {
                if let Some(error) = Retained::retain(error) {
                    Err(Error::Provider(error).into())
                } else if let Some(data) = data.as_ref() {
                    if data.len() > limit {
                        Err(Error::Size { limit }.into())
                    } else {
                        Ok(Loaded {
                            format,
                            payload: Some(Payload::Data(data.to_vec())),
                        })
                    }
                } else {
                    Ok(Loaded {
                        format,
                        payload: None,
                    })
                }
            };
            let _ = send.try_send(result);
        });
        // SAFETY: the callback captures only synchronized Send + Sync state.
        // NSItemProvider access itself remains on the main thread.
        let progress = unsafe {
            candidate
                .provider
                .loadDataRepresentationForTypeIdentifier_completionHandler(
                    kind,
                    &completion,
                )
        };
        // Also handles an inline completion or cancellation before native load
        // returns its progress: install then cannot leave uncancelled progress.
        attempt.install(progress);
    });
    loop {
        if cancelled() {
            return Err(Error::Cancelled.into());
        }
        let now = Instant::now();
        if now >= deadline {
            return Err(Error::Timeout.into());
        }
        // Timed channel waits observe upload cancellation/target invalidation
        // within 10ms even when a provider never calls its completion.
        match reply.recv_timeout((deadline - now).min(Duration::from_millis(10))) {
            Ok(result) => {
                let loaded = result?;
                if cancelled() {
                    return Err(Error::Cancelled.into());
                }
                if Instant::now() >= deadline {
                    return Err(Error::Timeout.into());
                }
                return Ok(loaded);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return Err(Error::ProviderClosed.into()),
        }
    }
}

/// Callback arguments must be valid for this call; no borrowed pointer escapes.
unsafe fn object_result(
    object: *mut ProtocolObject<dyn NSItemProviderReading>,
    error: *mut NSError,
    kind: ObjectKind,
) -> Result<Loaded> {
    // SAFETY: caller establishes callback argument validity. Retain the native
    // source or class-checked UIImage before crossing to the worker.
    unsafe {
        if let Some(error) = Retained::retain(error) {
            return Err(Error::Provider(error).into());
        }
        let payload = match object.as_ref() {
            None => None,
            Some(object) => {
                let object: &AnyObject = object.as_ref();
                match kind {
                    ObjectKind::Image => {
                        let image = object.downcast_ref::<UIImage>().ok_or(Error::ImageObject)?;
                        Retained::retain(image as *const UIImage as *mut UIImage)
                            .map(Payload::Image)
                    }
                    ObjectKind::Text => {
                        let text = object
                            .downcast_ref::<NSString>()
                            .ok_or(Error::ProviderObject)?;
                        if text.len() > super::super::TEXT_LIMIT {
                            return Err(Error::Size {
                                limit: super::super::TEXT_LIMIT,
                            }
                            .into());
                        }
                        Some(Payload::Data(text.to_string().into_bytes()))
                    }
                }
            }
        };
        Ok(Loaded {
            format: match kind {
                ObjectKind::Image => Some(ImageFormat::Png),
                ObjectKind::Text => None,
            },
            payload,
        })
    }
}
