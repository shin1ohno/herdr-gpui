//! Sessions the host or the network ends first (adversarial review B1, S1).
//! The failure classification then observes the end of russh's session
//! task through its handle, and the teardown must count that as joined
//! rather than poll the completed join again: that poll panics (tokio's
//! oneshot, "called after complete") and turned the receipt into
//! `Unproven`. `Mutant::PollJoinTwice` restores the first head's double
//! poll, so its test shows that these tests fail on that code.

use super::*;
use crate::{
    hooks::Event,
    test_host::{Behavior, TestHost},
};

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

/// The host drops the connection while answering authentication. The
/// fixture must reach the failed-authentication branch (`AuthEnded`) and
/// observe the session's end there (`SessionEnded`), before the exit is
/// recorded and so before the teardown starts; then the teardown must count
/// it as joined.
fn auth_stage_check(mutant: Option<Mutant>) -> Result<(), Violation> {
    let host = TestHost::start(Behavior::HangUpAtAuth);
    let mut run = start_watched(
        &target(host.port),
        &host.pins,
        watched_key(71, None),
        "ended during authentication",
        Deadlines::default(),
        removing(mutant),
    );
    run.stream.read_to_end(&mut Vec::new()).map_err(fixture)?;
    let settled = receipt(&run.tunnel);
    let events = run.probe.events();
    let auth = position(&events, Event::AuthEnded)
        .ok_or_else(|| fixture(format!("authentication did not fail: {events:?}")))?;
    let exit = position(&events, Event::ExitRecorded)
        .ok_or_else(|| fixture(format!("no exit recorded: {events:?}")))?;
    match position(&events, Event::SessionEnded) {
        Some(ended) if auth < ended && ended < exit => {}
        _ => return Err(Violation::JoinNotObservedAtAuth),
    }
    let _ = settled?;
    released_at_receipt(&run.probe)
}

#[test]
fn a_connection_dropped_during_authentication_is_observed_there() {
    holds(auth_stage_check(None));
}

#[test]
fn skipping_the_session_check_after_a_failed_authentication_is_caught() {
    assert_eq!(
        auth_stage_check(Some(Mutant::SkipAuthSessionError)),
        Err(Violation::JoinNotObservedAtAuth)
    );
}

#[test]
fn polling_the_join_twice_after_a_dropped_authentication_is_caught() {
    assert_eq!(
        host_ended_check(Behavior::HangUpAtAuth, Some(Mutant::PollJoinTwice)),
        Err(Violation::UnprovenInsteadOfComplete)
    );
}
