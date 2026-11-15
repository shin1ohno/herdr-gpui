//! The open stage (re-review P2-1 and S4) and R5. A host that stops before
//! its identification line, or after it in the key exchange, still yields a
//! receipt on a stop and on the connect deadline: the cut stops the stream
//! and drives the handshake to its end instead of dropping it, so a session
//! task russh spawned during the key exchange is joined. A handshake that
//! still does not end leaves the tunnel unjoined, which is `Unproven`. A
//! stop during authentication sends no exec request.

use super::*;
use crate::{
    hooks::Event,
    test_host::{Behavior, Seen, StalledPeer, TestHost},
};

#[derive(Debug, Clone, Copy)]
enum Cut {
    Stop,
    Deadline,
}

fn stalled_run(
    peer: &StalledPeer,
    cut: Cut,
    deadlines: Deadlines,
    hooks: Hooks,
) -> Result<Watched, Violation> {
    let pin =
        HostKey::from_known_hosts("ssh-ed25519", ED25519).ok_or_else(|| fixture("bad test pin"))?;
    let deadlines = match cut {
        Cut::Stop => deadlines,
        Cut::Deadline => Deadlines {
            connect: Duration::from_millis(400),
            ..deadlines
        },
    };
    let run = start_watched(
        &target(peer.port),
        &Pins::Keys(vec![pin]),
        watched_key(50, None),
        "never",
        deadlines,
        hooks,
    );
    if !TestHost::wait_for(&peer.accepted, 1, SETTLE) {
        return Err(fixture("the peer never accepted"));
    }
    if let Cut::Stop = cut {
        // Long enough for the client to be waiting on the stalled peer.
        std::thread::sleep(Duration::from_millis(100));
        run.tunnel.stop();
    }
    Ok(run)
}

fn expected_exit(cut: Cut) -> Option<TunnelExit> {
    Some(TunnelExit::Ssh(match cut {
        Cut::Stop => Failure::Cancelled,
        Cut::Deadline => Failure::Timeout,
    }))
}

fn stalled_check(identify: bool, cut: Cut, mutant: Option<Mutant>) -> Result<(), Violation> {
    let peer = StalledPeer::start(identify);
    let run = stalled_run(&peer, cut, Deadlines::default(), removing(mutant))?;
    let receipt = receipt(&run.tunnel)?;
    if receipt.tunnel() != run.tunnel.id() {
        return Err(fixture("the receipt names another tunnel"));
    }
    let exit = run.tunnel.wait_exit(Duration::ZERO);
    if exit != expected_exit(cut) {
        return Err(fixture(format!("unexpected exit {exit:?}")));
    }
    if position(&run.probe.events(), Event::ExecSent).is_some() {
        return Err(fixture("an exec request was queued"));
    }
    // The connection the session task owned was closed, not abandoned.
    if !TestHost::wait_for(&peer.closed, 1, SETTLE) {
        return Err(fixture("the client never closed the stalled connection"));
    }
    Ok(())
}

// Before the identification line russh has not spawned the session task
// yet, so these two pass even if the handshake future were dropped; the
// key-exchange cases below are the ones that tell the rule apart.

#[test]
fn a_stop_before_the_identification_line_yields_a_receipt() {
    holds(stalled_check(false, Cut::Stop, None));
}

#[test]
fn the_deadline_before_the_identification_line_yields_a_receipt() {
    holds(stalled_check(false, Cut::Deadline, None));
}

#[test]
fn a_stop_during_the_key_exchange_yields_a_receipt() {
    holds(stalled_check(true, Cut::Stop, None));
}

#[test]
fn the_deadline_during_the_key_exchange_yields_a_receipt() {
    holds(stalled_check(true, Cut::Deadline, None));
}

/// Dropping the handshake future leaves russh's session task running in
/// the key exchange, holding the stream, so no receipt can follow.
#[test]
fn dropping_the_handshake_on_a_stop_is_caught() {
    assert_eq!(
        stalled_check(true, Cut::Stop, Some(Mutant::DropConnectingOnCut)),
        Err(Violation::UnprovenInsteadOfComplete)
    );
}

#[test]
fn dropping_the_handshake_on_the_deadline_is_caught() {
    assert_eq!(
        stalled_check(true, Cut::Deadline, Some(Mutant::DropConnectingOnCut)),
        Err(Violation::UnprovenInsteadOfComplete)
    );
}

/// The `unjoined` branch: a handshake that does not end within the join
/// deadline after the cut (here the stream ignores the kill) is `Unproven`.
fn unjoined_check(cut: Cut) -> Result<(), Violation> {
    let peer = StalledPeer::start(true);
    let deadlines = Deadlines {
        join: Duration::from_millis(300),
        ..Deadlines::default()
    };
    let hooks = Hooks {
        deaf_transport: true,
        ..Hooks::default()
    };
    let run = stalled_run(&peer, cut, deadlines, hooks)?;
    unproven(&run.tunnel)?;
    let exit = run.tunnel.wait_exit(Duration::ZERO);
    if exit != expected_exit(cut) {
        return Err(fixture(format!("unexpected exit {exit:?}")));
    }
    Ok(())
}

#[test]
fn a_handshake_that_outlives_the_cut_on_a_stop_is_unproven() {
    holds(unjoined_check(Cut::Stop));
}

#[test]
fn a_handshake_that_outlives_the_cut_on_the_deadline_is_unproven() {
    holds(unjoined_check(Cut::Deadline));
}

/// R5: a stop while the host holds the authentication reply sends no exec
/// request, even once the host answers.
fn r5_check(mutant: Option<Mutant>) -> Result<(), Violation> {
    let host = TestHost::holding_auth(Behavior::Exit {
        stdout: b"late".to_vec(),
        status: 0,
    });
    let run = start_watched(
        &target(host.port),
        &host.pins,
        watched_key(51, None),
        "after stop",
        Deadlines::default(),
        removing(mutant),
    );
    if !TestHost::wait_for(&host.seen.auths, 1, SETTLE) {
        return Err(fixture("authentication never reached the host"));
    }
    run.tunnel.stop();
    std::thread::sleep(Duration::from_millis(100));
    host.answer_auth();
    host.answer_auth();
    let _ = receipt(&run.tunnel)?;
    if TestHost::wait_for(&host.seen.execs, 1, Duration::from_millis(500)) {
        return Err(Violation::ExecAfterStop);
    }
    if Seen::get(&host.seen.connections) != 1 {
        return Err(fixture("unexpected connection count"));
    }
    Ok(())
}

#[test]
fn r5_a_stop_during_authentication_sends_no_exec() {
    holds(r5_check(None));
}

#[test]
fn r5_ignoring_a_stop_during_setup_is_caught() {
    assert_eq!(
        r5_check(Some(Mutant::IgnoreStopInSetup)),
        Err(Violation::ExecAfterStop)
    );
}
