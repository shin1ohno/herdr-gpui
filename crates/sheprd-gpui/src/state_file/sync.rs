//! Waiting until a writer's latest value is on disk. iPadOS may end a
//! backgrounded app without a quit, so it cannot rely on the stores'
//! `on_app_quit` hooks, which also stop their writers. Every writer registers
//! here instead, and `sync_all` waits for whatever each one was handed.

use std::{
    sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak},
    time::{Duration, Instant},
};

/// Generations of the values handed to one writer.
#[derive(Debug, Default)]
struct Progress {
    /// The generation of the latest value `save` handed over.
    queued: u64,
    /// The generation of the latest value the worker attempted to write,
    /// whether or not the write succeeded.
    written: u64,
    /// The worker is gone, so nothing more will be written.
    finished: bool,
}

/// The progress a writer and its worker share, and the condition a sync
/// waits on.
#[derive(Debug)]
pub(super) struct Barrier {
    changed: Condvar,
    name: String,
    progress: Mutex<Progress>,
}

impl Barrier {
    pub(super) fn new(name: &str) -> Self {
        Self {
            changed: Condvar::new(),
            name: name.to_owned(),
            progress: Mutex::default(),
        }
    }

    /// Plain counters stay consistent even when a holder panicked.
    fn lock(&self) -> MutexGuard<'_, Progress> {
        self.progress.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The generation of a value about to be handed to the worker. Callers
    /// hold the writer's mailbox while calling, so the worker cannot take a
    /// value whose generation is not yet counted.
    pub(super) fn queue(&self) -> u64 {
        let mut progress = self.lock();
        progress.queued += 1;
        progress.queued
    }

    /// The worker attempted to write the value of `generation`.
    pub(super) fn written(&self, generation: u64) {
        let mut progress = self.lock();
        progress.written = progress.written.max(generation);
        drop(progress);
        self.changed.notify_all();
    }

    /// The worker ended and writes nothing more.
    pub(super) fn finish(&self) {
        self.lock().finished = true;
        self.changed.notify_all();
    }

    /// Waits until every value handed over before the call was attempted, or
    /// the worker ended, or `deadline` passed; `None` waits without one.
    /// Blocks, so callers run it off the UI thread.
    pub(super) fn sync_until(&self, deadline: Option<Instant>) -> Synced {
        let mut progress = self.lock();
        let target = progress.queued;
        loop {
            if progress.written >= target {
                return Synced::Written;
            }
            if progress.finished {
                return Synced::Lost;
            }
            let Some(deadline) = deadline else {
                progress = self
                    .changed
                    .wait(progress)
                    .unwrap_or_else(PoisonError::into_inner);
                continue;
            };
            let Some(left) = deadline
                .checked_duration_since(Instant::now())
                .filter(|left| !left.is_zero())
            else {
                return Synced::TimedOut;
            };
            progress = match self.changed.wait_timeout(progress, left) {
                Ok((progress, _)) => progress,
                Err(poisoned) => poisoned.into_inner().0,
            };
        }
    }
}

/// How waiting for one writer ended.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Synced {
    /// Every value handed over before the wait was written, or failed to.
    Written,
    /// The worker ended, by a panic, before attempting the latest value, so
    /// that value never reaches the disk.
    Lost,
    /// The deadline passed while a value was still being written.
    TimedOut,
}

/// Marks the barrier finished when the worker ends, also by panic, so a sync
/// never waits on a worker that is gone.
pub(super) struct FinishOnDrop(pub(super) Arc<Barrier>);

impl Drop for FinishOnDrop {
    fn drop(&mut self) {
        self.0.finish();
    }
}

/// How a `sync_all` went: writers whose latest value was attempted, writers
/// whose worker ended without it, and writers still writing at the deadline.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct SyncReport {
    pub(crate) lost: usize,
    pub(crate) synced: usize,
    pub(crate) timed_out: usize,
}

/// Live writers, held weakly so a dropped writer is not kept alive.
#[derive(Debug, Default)]
pub(super) struct Registry(Mutex<Vec<Weak<Barrier>>>);

impl Registry {
    pub(super) const fn new() -> Self {
        Self(Mutex::new(Vec::new()))
    }

    fn lock(&self) -> MutexGuard<'_, Vec<Weak<Barrier>>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn register(&self, barrier: &Arc<Barrier>) {
        let mut writers = self.lock();
        writers.retain(|writer| writer.strong_count() > 0);
        writers.push(Arc::downgrade(barrier));
    }

    /// The live writers, after dropping the entries of writers that are gone.
    fn live(&self) -> Vec<Arc<Barrier>> {
        let mut writers = self.lock();
        writers.retain(|writer| writer.strong_count() > 0);
        writers.iter().filter_map(Weak::upgrade).collect()
    }

    /// Waits for every live writer within one overall `timeout`; a timeout
    /// too large to add to now waits without a deadline. The registry is not
    /// locked while waiting, so writers may start meanwhile.
    pub(super) fn sync(&self, timeout: Duration) -> SyncReport {
        let deadline = Instant::now().checked_add(timeout);
        let mut report = SyncReport::default();
        for writer in self.live() {
            match writer.sync_until(deadline) {
                Synced::Written => report.synced += 1,
                Synced::Lost => {
                    tracing::warn!(
                        file = writer.name,
                        "State was not saved: its writer ended before writing it"
                    );
                    report.lost += 1;
                }
                Synced::TimedOut => {
                    tracing::warn!(
                        file = writer.name,
                        "State was still being saved at the deadline"
                    );
                    report.timed_out += 1;
                }
            }
        }
        report
    }

    /// Whether `barrier`'s writer is registered and live.
    #[cfg(test)]
    pub(super) fn contains(&self, barrier: &Arc<Barrier>) -> bool {
        self.live().iter().any(|live| Arc::ptr_eq(live, barrier))
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.live().len()
    }
}

/// Every writer in the process.
pub(super) static REGISTRY: Registry = Registry::new();

/// Waits up to `timeout` for every live writer in the process to write the
/// latest value it was handed. Blocks, so callers run it on the background
/// executor or a worker thread, never the UI thread.
#[cfg_attr(
    not(any(target_os = "ios", test)),
    expect(dead_code, reason = "only iPadOS flushes stores outside quit")
)]
pub(crate) fn sync_all(timeout: Duration) -> SyncReport {
    REGISTRY.sync(timeout)
}
