//! R4, R7, R8: a panicking task, a dropped tunnel, and a session task that
//! outlives its deadline.

use super::*;
use crate::test_host::{Behavior, StalledPeer, TestHost};

fn test_pins() -> Result<Pins, Violation> {
    let pin =
        HostKey::from_known_hosts("ssh-ed25519", ED25519).ok_or_else(|| fixture("bad test pin"))?;
    Ok(Pins::Keys(vec![pin]))
}

/// R4: a task that panics never reports `Complete`, and records no exit.
fn r4_check(mutant: Option<Mutant>) -> Result<(), Violation> {
    let host = TestHost::start(Behavior::Exit {
        stdout: b"x".to_vec(),
        status: 0,
    });
    let hooks = Hooks {
        pump_panic: true,
        ..removing(mutant)
    };
    let run = start_watched(
        &target(host.port),
        &host.pins,
        watched_key(40, None),
        "panic",
        Deadlines::default(),
        hooks,
    );
    unproven(&run.tunnel)?;
    let exit = run.tunnel.wait_exit(Duration::ZERO);
    if exit.is_some() {
        return Err(fixture(format!("a panicked task recorded {exit:?}")));
    }
    Ok(())
}

#[test]
fn r4_a_panicking_task_is_unproven() {
    holds(r4_check(None));
}

#[test]
fn r4_completing_after_a_panic_is_caught() {
    assert_eq!(
        r4_check(Some(Mutant::CompleteOnPanic)),
        Err(Violation::CompleteInsteadOfUnproven)
    );
}

/// R7: dropping a tunnel never waits for its teardown, here made slow.
fn r7_check(mutant: Option<Mutant>) -> Result<(), Violation> {
    // Accepts and never speaks, so the tunnel stays in its handshake.
    let peer = StalledPeer::start(false);
    let hooks = Hooks {
        teardown_delay: Some(Duration::from_millis(600)),
        ..removing(mutant)
    };
    let run = start_watched(
        &target(peer.port),
        &test_pins()?,
        watched_key(41, None),
        "drop",
        Deadlines::default(),
        hooks,
    );
    let Watched {
        mut stream, tunnel, ..
    } = run;
    let dropping = Instant::now();
    drop(tunnel);
    let elapsed = dropping.elapsed();
    if elapsed > Duration::from_millis(200) {
        return Err(Violation::DropBlocked);
    }
    // Cancelling drops the runtime's end, so the caller reads end-of-stream.
    let read = stream.read_to_end(&mut Vec::new()).map_err(fixture)?;
    if read != 0 {
        return Err(fixture(format!(
            "read {read} bytes from a tunnel that never ran"
        )));
    }
    Ok(())
}

#[test]
fn r7_dropping_a_tunnel_does_not_wait_for_its_teardown() {
    holds(r7_check(None));
}

#[test]
fn r7_a_blocking_drop_is_caught() {
    assert_eq!(
        r7_check(Some(Mutant::BlockingDrop)),
        Err(Violation::DropBlocked)
    );
}

/// R8: a session task that has not ended by the deadline makes the teardown
/// `Unproven`, and the tunnel still drops its key and pins (condition 4 on
/// that path).
fn r8_check(mutant: Option<Mutant>) -> Result<(), Violation> {
    let host = TestHost::start(Behavior::Silent);
    let deadlines = Deadlines {
        disconnect: Duration::from_millis(100),
        join: Duration::from_millis(300),
        ..Deadlines::default()
    };
    let hooks = Hooks {
        join_delay: Some(Duration::from_secs(3)),
        ..removing(mutant)
    };
    let run = start_watched(
        &target(host.port),
        &host.pins,
        watched_key(42, None),
        "late",
        deadlines,
        hooks,
    );
    if !TestHost::wait_for(&host.seen.execs, 1, SETTLE) {
        return Err(fixture("the exec request never arrived"));
    }
    run.tunnel.stop();
    unproven(&run.tunnel)?;
    let alive = run
        .probe
        .alive_at_settle()
        .ok_or_else(|| fixture("no settle observed"))?;
    if alive.key != 0 {
        return Err(Violation::KeyHeld);
    }
    Ok(())
}

#[test]
fn r8_a_session_task_past_its_deadline_is_unproven() {
    holds(r8_check(None));
}

#[test]
fn r8_completing_after_the_deadline_is_caught() {
    assert_eq!(
        r8_check(Some(Mutant::CompleteOnJoinTimeout)),
        Err(Violation::CompleteInsteadOfUnproven)
    );
}
