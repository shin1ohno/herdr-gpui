use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A label no other test or run uses, whose key is deleted on test exit.
pub(super) struct Label(pub(super) String);

impl Label {
    pub(super) fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        Self(format!(
            "dev.herdr.gpui.test.device-key.{}.{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }
}

impl Drop for Label {
    fn drop(&mut self) {
        let _ = DeviceKey::delete(&self.0);
    }
}
