//! Sessions the host or the network ends first (adversarial review B1, S1).
//! The failure classification then observes the end of russh's session
//! task through its handle, and the teardown must count that as joined
//! rather than poll the completed join again: that poll panics (tokio's
//! oneshot, "called after complete") and turned the receipt into
//! `Unproven`. `Mutant::PollJoinTwice` restores the first head's double
//! poll, so its test shows that these tests fail on that code.

use super::*;
use crate::test_host::{Behavior, TestHost};

fn host_ended_check(behavior: Behavior, mutant: Option<Mutant>) -> Result<(), Violation> {
    let host = TestHost::start(behavior);
    let mut run = start_watched(
        &target(host.port),
        &host.pins,
        watched_key(70, None),
        "ended by the host",
        Deadlines::default(),
        removing(mutant),
    );
    // No stop: the host's end alone ends the tunnel.
    run.stream.read_to_end(&mut Vec::new()).map_err(fixture)?;
    let _ = receipt(&run.tunnel)?;
    let exit = run.tunnel.wait_exit(Duration::ZERO);
    if !matches!(exit, Some(TunnelExit::Ssh(_))) {
        return Err(fixture(format!("expected an SSH failure, got {exit:?}")));
    }
    released_at_receipt(&run.probe)?;
    if !TestHost::wait_for(&host.seen.ended, 1, SETTLE) {
        return Err(fixture("the host's session never ended"));
    }
    Ok(())
}

#[test]
fn a_connection_dropped_after_the_exec_yields_a_receipt() {
    holds(host_ended_check(Behavior::HangUpAfterExec, None));
}

#[test]
fn a_host_disconnect_after_the_exec_yields_a_receipt() {
    holds(host_ended_check(Behavior::DisconnectAfterExec, None));
}

#[test]
fn a_connection_dropped_during_authentication_yields_a_receipt() {
    holds(host_ended_check(Behavior::HangUpAtAuth, None));
}

#[test]
fn polling_the_join_twice_after_a_dropped_connection_is_caught() {
    assert_eq!(
        host_ended_check(Behavior::HangUpAfterExec, Some(Mutant::PollJoinTwice)),
        Err(Violation::UnprovenInsteadOfComplete)
    );
}

#[test]
fn polling_the_join_twice_after_a_host_disconnect_is_caught() {
    assert_eq!(
        host_ended_check(Behavior::DisconnectAfterExec, Some(Mutant::PollJoinTwice)),
        Err(Violation::UnprovenInsteadOfComplete)
    );
}
