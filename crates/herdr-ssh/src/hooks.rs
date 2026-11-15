//! Test-only fault points, rule removals, and observations for the tunnel's
//! teardown. Outside `cfg(test)` [`Hooks`] is an empty struct whose methods
//! return constants, so every call site compiles to the production path.
//!
//! A [`Mutant`] removes one teardown rule. Each rule's test runs the same
//! scenario with and without its mutant and asserts that its check passes
//! without it and fails with it, so CI shows both that the rule holds and
//! that the test would catch its removal.

#[cfg(test)]
use std::sync::{Mutex, PoisonError, Weak};
use std::{sync::Arc, time::Duration};

/// One removed rule; see the receipt tests for the rule each one breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) enum Mutant {
    // keep-sorted start
    /// R7: dropping a tunnel waits for its teardown.
    BlockingDrop,
    /// R8: a session task that outlives its deadline still yields `Complete`.
    CompleteOnJoinTimeout,
    /// R4: a task that ends without settling reports `Complete`.
    CompleteOnPanic,
    /// R3: a signing closure that outlives its deadline still yields `Complete`.
    CompleteOnSignerTimeout,
    /// R9: `exec` reaches the host before refusing a target without pins.
    ConnectBeforePinCheck,
    /// R2: the teardown does not wait for running signing closures.
    IgnoreSigners,
    /// R5: a stop during authentication or channel setup is not observed.
    IgnoreStopInSetup,
    /// Condition 2: the input forwarder is not ended before the receipt.
    KeepInputForwarder,
    /// Condition 4: the request (key and pins) outlives the receipt.
    KeepRequest,
    /// R11: the teardown log names the target.
    LogTarget,
    /// R6: the receipt is settled before the exit is recorded.
    ReceiptBeforeExit,
    /// R1: the receipt is settled without joining the session task.
    ReceiptBeforeJoin,
    // keep-sorted end
}

/// A point in the tunnel's life that a test can order against others.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) enum Event {
    // keep-sorted start
    /// The exec request is about to be queued on the channel.
    ExecSent,
    /// The tunnel's exit was given to `wait_exit`.
    ExitRecorded,
    /// The teardown settled `Complete`.
    Receipt,
    /// The teardown settled `Unproven`.
    Unproven,
    // keep-sorted end
}

/// Something the tunnel hands a stand-in to, so a test can tell whether that
/// holder still exists when the teardown settles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) enum Holder {
    /// The task copying the caller's writes to the command's input.
    Forwarder,
    /// russh's handler, which lives in the session task and holds the pins.
    Handler,
}

#[cfg(test)]
#[derive(Debug, Clone, Default)]
pub(crate) struct Hooks {
    pub(crate) mutant: Option<Mutant>,
    pub(crate) probe: Option<Arc<Probe>>,
    /// Panic in the pump on the first channel message.
    pub(crate) pump_panic: bool,
    /// Added to the wait for the session task, as if it ended late.
    pub(crate) join_delay: Option<Duration>,
    /// Waited before the teardown starts.
    pub(crate) teardown_delay: Option<Duration>,
}

#[cfg(not(test))]
#[derive(Debug, Clone, Default)]
pub(crate) struct Hooks;

#[cfg(test)]
impl Hooks {
    pub(crate) fn removes(&self, rule: Mutant) -> bool {
        self.mutant == Some(rule)
    }

    pub(crate) fn record(&self, event: Event) {
        if let Some(probe) = &self.probe {
            probe.record(event);
        }
    }

    pub(crate) fn holder(&self, holder: Holder) -> Option<Arc<()>> {
        self.probe.as_ref().map(|probe| match holder {
            Holder::Forwarder => Arc::clone(&probe.forwarder),
            Holder::Handler => Arc::clone(&probe.handler),
        })
    }

    pub(crate) fn pump_panics(&self) -> bool {
        self.pump_panic
    }

    pub(crate) fn join_delay(&self) -> Option<Duration> {
        self.join_delay
    }

    pub(crate) fn teardown_delay(&self) -> Option<Duration> {
        self.teardown_delay
    }
}

#[cfg(not(test))]
impl Hooks {
    pub(crate) fn removes(&self, _rule: Mutant) -> bool {
        false
    }

    pub(crate) fn record(&self, _event: Event) {}

    pub(crate) fn holder(&self, _holder: Holder) -> Option<Arc<()>> {
        None
    }

    pub(crate) fn pump_panics(&self) -> bool {
        false
    }

    pub(crate) fn join_delay(&self) -> Option<Duration> {
        None
    }

    pub(crate) fn teardown_delay(&self) -> Option<Duration> {
        None
    }
}

/// How many of each holder were alive when the teardown settled.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Alive {
    pub(crate) forwarder: usize,
    pub(crate) handler: usize,
    /// Copies of the client key's signing closure.
    pub(crate) key: usize,
}

/// What a test observed of one tunnel.
#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct Probe {
    events: Mutex<Vec<Event>>,
    forwarder: Arc<()>,
    handler: Arc<()>,
    key: Mutex<Option<Weak<()>>>,
    at_settle: Mutex<Option<Alive>>,
}

#[cfg(test)]
impl Probe {
    /// Watches a stand-in that only the client key's signing closure holds.
    pub(crate) fn watch_key(&self, key: Weak<()>) {
        *self.key.lock().unwrap_or_else(PoisonError::into_inner) = Some(key);
    }

    pub(crate) fn events(&self) -> Vec<Event> {
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn alive_at_settle(&self) -> Option<Alive> {
        *self
            .at_settle
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    fn record(&self, event: Event) {
        if matches!(event, Event::Receipt | Event::Unproven) {
            let key = self
                .key
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .as_ref()
                .map_or(0, Weak::strong_count);
            let alive = Alive {
                // The probe's own reference is not a holder.
                forwarder: Arc::strong_count(&self.forwarder) - 1,
                handler: Arc::strong_count(&self.handler) - 1,
                key,
            };
            *self
                .at_settle
                .lock()
                .unwrap_or_else(PoisonError::into_inner) = Some(alive);
        }
        self.events
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(event);
    }
}
