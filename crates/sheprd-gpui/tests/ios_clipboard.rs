//! A small native executable: libtest runs tests on worker threads, while
//! UIKit pasteboards must be tested on the process's main thread.

#[cfg(target_os = "ios")]
#[path = "../src/window/clipboard/error.rs"]
mod error;
#[cfg(target_os = "ios")]
pub use error::Error;
#[cfg(target_os = "ios")]
#[derive(Debug, thiserror::Error)]
enum TestError {
    #[error(transparent)]
    Clipboard(#[from] Error),
}
#[cfg(target_os = "ios")]
type Result<T> = std::result::Result<T, TestError>;

// The selection helper accepts the reader's acquisition bounds. Small test
// bounds suffice here; this exercises UIKit representation selection, not the
// application's upload limit.
#[cfg(target_os = "ios")]
const TEXT_LIMIT: usize = 64;
#[cfg(target_os = "ios")]
const IMAGE_LIMIT: usize = 1024;
#[cfg(target_os = "ios")]
const IMAGE_PIXEL_LIMIT: u64 = 64;
#[cfg(target_os = "ios")]
const MAX_ACQUISITIONS: usize = 4;
#[cfg(target_os = "ios")]
#[path = "../src/window/clipboard/acquire.rs"]
mod acquire;
#[cfg(target_os = "ios")]
use acquire::read_with;
#[cfg(target_os = "ios")]
#[path = "../src/window/clipboard/ios.rs"]
mod native;
#[cfg(target_os = "ios")]
use native::*;
#[cfg(target_os = "ios")]
#[path = "../src/window/clipboard/ios/tests.rs"]
mod tests;

#[cfg(target_os = "ios")]
#[path = "ios_clipboard/app.rs"]
mod app;

fn main() {
    #[cfg(not(target_os = "ios"))]
    println!("Skipped UIKit clipboard regression: requires an iOS simulator");
    #[cfg(target_os = "ios")]
    app::run();
}

#[cfg(target_os = "ios")]
fn run_tests() {
    let _read = read;
    let checks: &[(&str, fn())] = &[
        (
            "nil text object same-provider data",
            tests::nil_text_object_uses_same_provider_data,
        ),
        (
            "mixed representations",
            tests::mixed_uikit_copy_preserves_normal_text_and_image_only_image,
        ),
        (
            "empty and oversized",
            tests::empty_uikit_copy_and_oversized_native_data_are_bounded,
        ),
        (
            "native error source",
            tests::provider_failure_preserves_the_native_source,
        ),
        (
            "nil/empty/provider fallback and empty text",
            tests::fallback_and_empty_text,
        ),
        (
            "delayed load main-runloop liveness",
            tests::delayed_provider_leaves_main_run_loop_live,
        ),
        (
            "denial stops fallback",
            tests::denial_stops_fallback_and_keeps_native_source,
        ),
        (
            "deadline and native cancellation",
            tests::cancellation_and_one_deadline_cancel_native_progress,
        ),
        (
            "provider metadata bound",
            tests::provider_limit_rejects_before_loading,
        ),
        (
            "late cancelled callback",
            tests::late_cancelled_callback_cannot_replace_a_later_read,
        ),
        (
            "UIImage and encoded PNG",
            tests::uiimage_and_encoded_png_are_loaded_through_the_native_board,
        ),
        // Deliberately noncompleting native work stays last; this process exits.
        (
            "native request ceiling",
            tests::ignored_cancellation_cannot_accumulate_native_requests,
        ),
    ];
    let mut passed = 0;
    for (name, check) in checks {
        if std::env::var("HERDR_TEST_CLIPBOARD_CASE").is_ok_and(|case| !name.contains(&case)) {
            continue;
        }
        eprintln!("UIKit clipboard: {name}");
        check();
        passed += 1;
    }
    assert!(passed > 0, "native test selector matched no cases");
    println!("UIKit clipboard native checks passed: {passed}");
}
