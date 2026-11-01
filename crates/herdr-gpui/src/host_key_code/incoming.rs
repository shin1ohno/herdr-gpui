//! The UIKit callback has no scene identity. Deliver to one frontmost main
//! window, or to the sole main window; never broadcast a code across scenes.

use crate::HerdrWindow;
use gpui::{App, WindowId};

pub(crate) fn open_urls(urls: Vec<String>, cx: &mut App) {
    // One callback may hold many URLs. Keeping only its first host-key URL
    // bounds both the work and the number of confirmation prompts it creates.
    let Some(url) = urls.into_iter().find(|url| url.starts_with("herdr-ipad:")) else {
        return;
    };
    let windows: Vec<_> = cx
        .windows()
        .into_iter()
        .filter_map(|handle| handle.downcast::<HerdrWindow>())
        .collect();
    let Some(id) = recipient(
        cx.active_window().map(|handle| handle.window_id()),
        windows.iter().map(|handle| handle.window_id()),
    ) else {
        return;
    };
    if let Some(handle) = windows.into_iter().find(|handle| handle.window_id() == id) {
        let _ = handle.update(cx, |view, window, cx| {
            view.receive_host_key_code_url(&url, window, cx);
        });
    }
}

fn recipient(
    active: Option<WindowId>,
    candidates: impl IntoIterator<Item = WindowId>,
) -> Option<WindowId> {
    let mut sole = None;
    let mut multiple = false;
    for id in candidates {
        if Some(id) == active {
            return Some(id);
        }
        multiple |= sole.replace(id).is_some();
    }
    if multiple { None } else { sole }
}

#[cfg(test)]
mod tests;
