//! Saving state when iPadOS moves the app to the background. The system may
//! end a backgrounded app without a quit, so the stores' `on_app_quit` hooks
//! cannot be relied on, and they stop their writers, which the app still
//! needs if it returns. This writes what is pending and leaves every store
//! running.

use std::time::{Duration, Instant};

use gpui::{App, FutureExt as _, Task};

use super::{SyncReport, sync_all};

/// How a background flush went.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Flushed {
    /// The Settings window's drafts finished writing, or there were none.
    pub(crate) drafts: bool,
    pub(crate) stores: SyncReport,
}

/// Writes the Settings window's unsaved drafts, as its "Save now" does, then
/// every state writer's latest value, all within `timeout`. The task resolves
/// only once both are on disk or the time is up, so a caller holding the app
/// awake until then covers both. The UI thread never blocks: drafts are
/// written by their own background tasks, and the stores are waited on from
/// the background executor.
pub(crate) fn flush(cx: &mut App, timeout: Duration) -> Task<Flushed> {
    let started = Instant::now();
    let drafts = crate::settings_window::save_open_drafts(cx);
    let executor = cx.background_executor().clone();
    cx.foreground_executor().spawn(async move {
        let drafts = drafts.with_timeout(timeout, &executor).await.is_ok();
        // After the drafts, so a store their completions saved to is waited
        // on too.
        let left = timeout.saturating_sub(started.elapsed());
        let stores = executor.spawn(async move { sync_all(left) }).await;
        Flushed { drafts, stores }
    })
}

#[cfg(test)]
mod tests;
