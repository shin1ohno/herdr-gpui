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
const IMAGE_LIMIT: usize = 64;
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

fn main() {
    #[cfg(not(target_os = "ios"))]
    println!("Skipped UIKit clipboard regression: requires an iOS simulator");
    #[cfg(target_os = "ios")]
    {
        // Also type-check the production general-pasteboard entry point,
        // without reading or changing the user's clipboard in this test.
        let _read = native::read;
        tests::mixed_uikit_copy_preserves_normal_text_and_image_only_image();
        tests::empty_uikit_copy_and_oversized_native_data_are_bounded();
        println!("UIKit mixed-format, empty clipboard, and acquisition bounds: passed");
    }
}
