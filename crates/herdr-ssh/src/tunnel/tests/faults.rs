//! R4, R7, R8: a panicking task, a dropped tunnel, and a session task that
//! outlives its deadline.

use std::io::Read as _;

use super::*;
use crate::test_host::{Behavior, StalledPeer, TestHost};

/// R4: a task that panics never reports `Complete`, and records no exit.
fn r4_check(mutant: Option<Mutant>) -> Result<(), String> {
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
    match run.tunnel.wait_teardown(SETTLE) {
        Some(Teardown::Unproven) => {}
        other => return Err(format!("expected unproven, got {other:?}")),
    }
    let exit = run.tunnel.wait_exit(Duration::ZERO);
    if exit.is_some() {
        return Err(format!("a panicked task recorded {exit:?}"));
    }
    Ok(())
}

#[test]
fn r4_a_panicking_task_is_unproven() {
    r4_check(None).unwrap();
}

#[test]
fn r4_completing_after_a_panic_is_caught() {
    assert!(r4_check(Some(Mutant::CompleteOnPanic)).is_err());
}

/// R7: dropping a tunnel never waits for its teardown, here made slow.
fn r7_check(mutant: Option<Mutant>) -> Result<(), String> {
    // Accepts and never speaks, so the tunnel stays in its handshake.
    let peer = StalledPeer::start(false);
    let hooks = Hooks {
        teardown_delay: Some(Duration::from_millis(600)),
        ..removing(mutant)
    };
    let run = start_watched(
        &target(peer.port),
        &Pins::Keys(vec![
            HostKey::from_known_hosts("ssh-ed25519", ED25519).ok_or("bad test pin")?,
        ]),
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
        return Err(format!("dropping the tunnel blocked for {elapsed:?}"));
    }
    // Cancelling drops the runtime's end, so the caller reads end-of-stream.
    let read = stream
        .read_to_end(&mut Vec::new())
        .map_err(|e| e.to_string())?;
    if read != 0 {
        return Err(format!("read {read} bytes from a tunnel that never ran"));
    }
    Ok(())
}

#[test]
fn r7_dropping_a_tunnel_does_not_wait_for_its_teardown() {
    r7_check(None).unwrap();
}

#[test]
fn r7_a_blocking_drop_is_caught() {
    assert!(r7_check(Some(Mutant::BlockingDrop)).is_err());
}

/// R8: a session task that has not ended by the deadline makes the teardown
/// `Unproven`, and the tunnel still drops its key and pins (condition 4 on
/// that path).
fn r8_check(mutant: Option<Mutant>) -> Result<(), String> {
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
        return Err("the exec request never arrived".into());
    }
    run.tunnel.stop();
    match run.tunnel.wait_teardown(SETTLE) {
        Some(Teardown::Unproven) => {}
        other => return Err(format!("expected unproven, got {other:?}")),
    }
    let alive = run.probe.alive_at_settle().ok_or("no settle observed")?;
    if alive.key != 0 {
        return Err(format!("the key outlived an unproven teardown: {alive:?}"));
    }
    Ok(())
}

#[test]
fn r8_a_session_task_past_its_deadline_is_unproven() {
    r8_check(None).unwrap();
}

#[test]
fn r8_completing_after_the_deadline_is_caught() {
    assert!(r8_check(Some(Mutant::CompleteOnJoinTimeout)).is_err());
}
