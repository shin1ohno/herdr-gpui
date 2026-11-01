//! Writing the open window's drafts when iPadOS moves the app to the
//! background, and telling the caller when they are on disk, so it can keep
//! the app awake until then.

use super::SettingsWindow;
use gpui::{Context, Task};
use std::time::Duration;

/// How often a background save checks whether the drafts have settled. The
/// chain of draft writes reports progress only through the window's own
/// state, and the caller bounds the whole wait with its deadline.
pub(super) const SETTLE_POLL: Duration = Duration::from_millis(20);

impl SettingsWindow {
    /// Starts writing the unsaved drafts as "Save now" does, also while a load
    /// or save is running: "Save now" is disabled then, but setting
    /// `flushing` lets that work's completion continue the chain through
    /// `finish_flush`. The task resolves once nothing is left to write or the
    /// window is gone, never sooner.
    pub(super) fn save_drafts_for_background(&mut self, cx: &mut Context<Self>) -> Task<()> {
        if !self.quitting {
            if self.busy() {
                self.theme_load_failed = false;
                self.flushing = true;
            } else {
                self.save_pending_drafts(cx);
            }
        }
        cx.spawn(async move |this, cx| {
            while this
                .read_with(cx, |view, _| !view.drafts_settled())
                .unwrap_or(false)
            {
                cx.background_executor().timer(SETTLE_POLL).await;
            }
        })
    }

    /// No "Save now" chain, load, save, or close is in progress, so nothing
    /// more will be written. Quitting writes the drafts through its own path.
    pub(super) fn drafts_settled(&self) -> bool {
        self.quitting || (!self.flushing && !self.busy() && self.closing.is_none())
    }
}
