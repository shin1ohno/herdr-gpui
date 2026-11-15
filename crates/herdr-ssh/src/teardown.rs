//! The end of a tunnel's local SSH work, as a receipt a caller can hold.
//!
//! [`Tunnel::wait_teardown`](crate::Tunnel::wait_teardown) returns
//! [`Teardown::Complete`] only when all of these hold for that tunnel:
//!
//! 1. Its exec request was never sent, or the session that carried it was
//!    sent a disconnect and russh's session task has ended and been joined.
//!    The task owns the TCP stream and shuts down its write side before it
//!    ends, so the host has been told. A deadline that cuts the connection
//!    short (the 10 s connect and 20 s setup deadlines included) still joins
//!    the task: the stream is cut and the handshake driven to its end rather
//!    than dropped, because only that future can join a task russh spawned
//!    during the handshake.
//! 2. The task that copies the caller's writes to the command's standard
//!    input was aborted and has ended, so it no longer holds the channel.
//! 3. No signing closure started for the tunnel is still running.
//! 4. The tunnel's copy of the client key and of the pins was dropped.
//!
//! It says nothing about the remote side: not that the remote command has
//! ended, not which bytes the host received, and not what the host logged.
//!
//! [`Teardown::Unproven`] means one of these could not be shown within its
//! deadline (5 s for the session task, 5 s for the signing closures), or the
//! tunnel's task ended without reaching the check (a panic). The tunnel's
//! copies of the key and pins are dropped on that path too, but a signing
//! closure that never returns keeps its own copy of the key.

use std::{
    pin::pin,
    sync::{
        Arc, Condvar, Mutex, PoisonError,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::Duration,
};

use tokio::sync::Notify;

mod error;

pub use error::Error;

/// A tunnel's identity within this process. Every tunnel the process starts
/// gets a new one, whichever host it reaches, so a receipt can be matched to
/// the attempt it ends and to no other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TunnelId(u64);

/// The next identity to hand out; it only grows, and runs out rather than
/// wrapping.
static NEXT_TUNNEL: AtomicU64 = AtomicU64::new(1);

pub(crate) fn next_tunnel_id() -> Result<TunnelId, Error> {
    allocate(&NEXT_TUNNEL)
}

fn allocate(counter: &AtomicU64) -> Result<TunnelId, Error> {
    counter
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
        .map(TunnelId)
        .map_err(|_| Error::IdsExhausted)
}

/// Proof that one tunnel's local SSH work has ended, in the sense of the
/// module documentation. Only this crate builds one, and it is not `Clone`.
///
/// ```compile_fail,E0451
/// fn forge(tunnel: herdr_ssh::TunnelId) -> herdr_ssh::TeardownReceipt {
///     herdr_ssh::TeardownReceipt { tunnel }
/// }
/// ```
///
/// The same types are reachable from outside, so the example above fails
/// only because the field is private:
///
/// ```
/// fn tunnel_of(receipt: &herdr_ssh::TeardownReceipt) -> herdr_ssh::TunnelId {
///     receipt.tunnel()
/// }
/// ```
#[derive(Debug)]
#[must_use]
pub struct TeardownReceipt {
    tunnel: TunnelId,
}

impl TeardownReceipt {
    /// The tunnel whose work ended; compare it with
    /// [`Tunnel::id`](crate::Tunnel::id) of the attempt being released.
    pub fn tunnel(&self) -> TunnelId {
        self.tunnel
    }
}

/// How a tunnel's local SSH work ended.
#[derive(Debug)]
#[must_use]
pub enum Teardown {
    Complete(TeardownReceipt),
    /// The end could not be shown; treat the tunnel as possibly running.
    Unproven,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Outcome {
    Complete,
    Unproven,
}

impl Outcome {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Unproven => "unproven",
        }
    }
}

/// Where the tunnel's task leaves its teardown outcome. The first outcome
/// stays, so a later one (the task's guard on its way out) cannot replace it.
#[derive(Debug, Default)]
pub(crate) struct TeardownSlot {
    outcome: Mutex<Option<Outcome>>,
    ended: Condvar,
}

impl TeardownSlot {
    pub(crate) fn settle(&self, outcome: Outcome) {
        let mut slot = self.outcome.lock().unwrap_or_else(PoisonError::into_inner);
        if slot.is_none() {
            *slot = Some(outcome);
        }
        drop(slot);
        self.ended.notify_all();
    }

    /// The outcome for `tunnel` once known, waiting at most `timeout`. Every
    /// call after the end returns the same outcome, each with its own receipt.
    pub(crate) fn wait(&self, tunnel: TunnelId, timeout: Duration) -> Option<Teardown> {
        let slot = self.outcome.lock().unwrap_or_else(PoisonError::into_inner);
        let (slot, _) = self
            .ended
            .wait_timeout_while(slot, timeout, |outcome| outcome.is_none())
            .unwrap_or_else(PoisonError::into_inner);
        let outcome = *slot;
        outcome.map(|outcome| match outcome {
            Outcome::Complete => Teardown::Complete(TeardownReceipt { tunnel }),
            Outcome::Unproven => Teardown::Unproven,
        })
    }
}

/// Settles the slot when the tunnel's task ends without having settled it
/// (an abort or a panic), so such an end is never mistaken for a pending one.
pub(crate) struct Fuse {
    slot: Arc<TeardownSlot>,
    on_drop: Outcome,
}

impl Fuse {
    /// `on_drop` is [`Outcome::Unproven`] outside tests.
    pub(crate) fn new(slot: Arc<TeardownSlot>, on_drop: Outcome) -> Self {
        Self { slot, on_drop }
    }

    pub(crate) fn settle(self, outcome: Outcome) {
        self.slot.settle(outcome);
    }
}

impl Drop for Fuse {
    fn drop(&mut self) {
        self.slot.settle(self.on_drop);
    }
}

/// The caller's request to stop, observed by the tunnel's task at its next
/// phase boundary.
#[derive(Debug, Default)]
pub(crate) struct StopSignal {
    stopped: AtomicBool,
    notify: Notify,
}

impl StopSignal {
    pub(crate) fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
        self.notify.notify_waiters();
    }

    pub(crate) fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::Acquire)
    }

    /// Resolves once [`StopSignal::stop`] has been called.
    pub(crate) async fn stopped(&self) {
        loop {
            let mut notified = pin!(self.notify.notified());
            // Registered before the flag is read, so a stop in between wakes it.
            let _ = notified.as_mut().enable();
            if self.is_stopped() {
                return;
            }
            notified.await;
        }
    }
}

/// Counts a tunnel's signing closures that have started and not returned.
#[derive(Debug, Default, Clone)]
pub(crate) struct SignTracker(Arc<SignCount>);

#[derive(Debug, Default)]
struct SignCount {
    running: AtomicUsize,
    idle: Notify,
}

impl SignTracker {
    /// Marks one closure as running until the returned guard drops.
    pub(crate) fn start(&self) -> Signing {
        self.0.running.fetch_add(1, Ordering::AcqRel);
        Signing(Arc::clone(&self.0))
    }

    /// Resolves once no closure is running.
    pub(crate) async fn idle(&self) {
        loop {
            let mut notified = pin!(self.0.idle.notified());
            let _ = notified.as_mut().enable();
            if self.0.running.load(Ordering::Acquire) == 0 {
                return;
            }
            notified.await;
        }
    }
}

/// One running signing closure; dropping it ends the count's claim.
#[derive(Debug)]
pub(crate) struct Signing(Arc<SignCount>);

impl Drop for Signing {
    fn drop(&mut self) {
        if self.0.running.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.0.idle.notify_waiters();
        }
    }
}

#[cfg(test)]
mod tests;
