//! The end of a tunnel's local SSH work: every path through the tunnel,
//! a normal exit included, ends here, and only this decides between
//! `Complete` and `Unproven` (the conditions are in `crate::teardown`).

use russh::{Disconnect, client::Handle};
use tokio::time::{Instant, sleep, timeout, timeout_at};

use super::{Controls, Request, Started};
use crate::{
    handler::Client,
    hooks::{Event, Mutant},
    teardown::{Outcome, SignTracker},
    transport::KillSwitch,
};

pub(super) async fn close(
    started: Started,
    request: Request,
    signers: &SignTracker,
    controls: &Controls,
) -> Outcome {
    let hooks = &controls.hooks;
    if let Some(delay) = hooks.teardown_delay() {
        sleep(delay).await;
    }
    if hooks.removes(Mutant::LogTarget) {
        tracing::debug!(
            category = "ssh",
            user = request.target.user.as_str(),
            host = request.target.host.as_str(),
            "ssh tunnel teardown started"
        );
    }
    let outcome = end_work(started, signers, controls).await;
    let mut request = Some(request);
    if !hooks.removes(Mutant::KeepRequest) {
        // Condition 4, on every outcome: the tunnel's copies of the key and
        // the pins go now, not whenever the task's future is dropped.
        request = None;
    }
    hooks.record(match outcome {
        Outcome::Complete => Event::Receipt,
        Outcome::Unproven => Event::Unproven,
    });
    tracing::debug!(
        category = "ssh",
        outcome = outcome.as_str(),
        "ssh tunnel teardown"
    );
    drop(request);
    outcome
}

/// Ends what the tunnel started, in order: the input forwarder (condition
/// 2), the session task (condition 1), then running signing closures
/// (condition 3). On `Unproven` the transport is cut, so a session task that
/// could not be joined still ends soon.
async fn end_work(mut started: Started, signers: &SignTracker, controls: &Controls) -> Outcome {
    let hooks = &controls.hooks;
    let deadlines = controls.deadlines;
    let by = Instant::now() + deadlines.join;
    let transport = started.transport.take();
    let give_up = || {
        if let Some(transport) = &transport {
            transport.kill();
        }
        Outcome::Unproven
    };
    // The forwarder holds the channel's write half, and so a sender into
    // the session task.
    if !hooks.removes(Mutant::KeepInputForwarder)
        && let Some(input) = started.input.take()
        && timeout_at(by, input.end()).await.is_err()
    {
        return give_up();
    }
    if started.unjoined {
        return give_up();
    }
    let skip_join = hooks.removes(Mutant::ReceiptBeforeJoin);
    let accept_late = hooks.removes(Mutant::CompleteOnJoinTimeout);
    if let Some(session) = started.session.take() {
        let joined = if skip_join {
            // The mutant drops the handle and does not wait for the task,
            // which ends on its own some time after the receipt.
            drop(session);
            true
        } else {
            join(session, transport.as_deref(), controls, by).await
        };
        if !joined && !accept_late {
            return give_up();
        }
    }
    // The session task owns the stream, so a stream not yet dropped means a
    // task that may still run.
    let proven_released = transport
        .as_ref()
        .is_none_or(|transport| transport.released());
    if !proven_released && !skip_join && !accept_late {
        return give_up();
    }
    if !hooks.removes(Mutant::IgnoreSigners)
        && timeout(deadlines.signers, signers.idle()).await.is_err()
        && !hooks.removes(Mutant::CompleteOnSignerTimeout)
    {
        return give_up();
    }
    drop(started);
    Outcome::Complete
}

/// Disconnects the session and waits, by `by`, for russh's session task to
/// end. A host that stops reading cannot hold this: the disconnect is queued
/// with a deadline, and after the grace the stream is cut.
///
/// Cutting the stream is not always enough. While the task waits for the
/// signature it asked for (an authentication the stop interrupted), it reads
/// neither the stream nor its messages, and discards the disconnect; only
/// losing every sender ends that wait (russh 0.65 `client/encrypted.rs`).
/// So the handle goes too, and the end is shown by the drop of the stream
/// the task owns: the runtime has one thread, so when this task sees the
/// drop, the session task's future has finished.
async fn join(
    mut session: Handle<Client>,
    transport: Option<&KillSwitch>,
    controls: &Controls,
    by: Instant,
) -> bool {
    let grace = (Instant::now() + controls.deadlines.disconnect).min(by);
    let _ = timeout_at(grace, session.disconnect(Disconnect::ByApplication, "", "")).await;
    let late = controls.hooks.join_delay();
    let joined = timeout_at(grace, async {
        if let Some(late) = late {
            sleep(late).await;
        }
        // The task's own result was already reported, or is not ours to report.
        let _ = (&mut session).await;
    })
    .await
    .is_ok();
    if joined {
        return true;
    }
    let Some(transport) = transport else {
        return false;
    };
    transport.kill();
    drop(session);
    timeout_at(by, async {
        if let Some(late) = late {
            sleep(late).await;
        }
        transport.wait_released().await;
    })
    .await
    .is_ok()
}
