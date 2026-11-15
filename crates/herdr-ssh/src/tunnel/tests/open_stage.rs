//! The open stage (re-review P2-1) and R5. A host that stops before its
//! identification line, or after it in the key exchange, still yields a
//! receipt on a stop and on the connect deadline: the deadline cuts the
//! stream and drives the handshake to its end instead of dropping it, so a
//! session task russh may have spawned is joined. A stop during
//! authentication sends no exec request.

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

fn stalled_check(identify: bool, cut: Cut) -> Result<(), String> {
    let peer = StalledPeer::start(identify);
    let deadlines = match cut {
        Cut::Stop => Deadlines::default(),
        Cut::Deadline => Deadlines {
            connect: Duration::from_millis(400),
            ..Deadlines::default()
        },
    };
    let run = start_watched(
        &target(peer.port),
        &Pins::Keys(vec![
            HostKey::from_known_hosts("ssh-ed25519", ED25519).ok_or("bad test pin")?,
        ]),
        watched_key(50, None),
        "never",
        deadlines,
        Hooks::default(),
    );
    if !TestHost::wait_for(&peer.accepted, 1, SETTLE) {
        return Err("the peer never accepted".into());
    }
    if let Cut::Stop = cut {
        // Long enough for the client to be waiting on the stalled peer.
        std::thread::sleep(Duration::from_millis(100));
        run.tunnel.stop();
    }
    let receipt = receipt(&run.tunnel)?;
    if receipt.tunnel() != run.tunnel.id() {
        return Err("the receipt names another tunnel".into());
    }
    let expected = match cut {
        Cut::Stop => Failure::Cancelled,
        Cut::Deadline => Failure::Timeout,
    };
    let exit = run.tunnel.wait_exit(Duration::ZERO);
    if exit != Some(TunnelExit::Ssh(expected)) {
        return Err(format!("unexpected exit {exit:?}"));
    }
    if position(&run.probe.events(), Event::ExecSent).is_some() {
        return Err("an exec request was queued".into());
    }
    // The connection the session task owned was closed, not abandoned.
    if !TestHost::wait_for(&peer.closed, 1, SETTLE) {
        return Err("the client never closed the stalled connection".into());
    }
    Ok(())
}

#[test]
fn a_stop_before_the_identification_line_yields_a_receipt() {
    stalled_check(false, Cut::Stop).unwrap();
}

#[test]
fn the_deadline_before_the_identification_line_yields_a_receipt() {
    stalled_check(false, Cut::Deadline).unwrap();
}

#[test]
fn a_stop_during_the_key_exchange_yields_a_receipt() {
    stalled_check(true, Cut::Stop).unwrap();
}

#[test]
fn the_deadline_during_the_key_exchange_yields_a_receipt() {
    stalled_check(true, Cut::Deadline).unwrap();
}

/// R5: a stop while the host holds the authentication reply sends no exec
/// request, even once the host answers.
fn r5_check(mutant: Option<Mutant>) -> Result<(), String> {
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
        return Err("authentication never reached the host".into());
    }
    run.tunnel.stop();
    std::thread::sleep(Duration::from_millis(100));
    host.answer_auth();
    host.answer_auth();
    let _ = receipt(&run.tunnel)?;
    if TestHost::wait_for(&host.seen.execs, 1, Duration::from_millis(500)) {
        return Err("the host received an exec request after the stop".into());
    }
    if Seen::get(&host.seen.connections) != 1 {
        return Err("unexpected connection count".into());
    }
    Ok(())
}

#[test]
fn r5_a_stop_during_authentication_sends_no_exec() {
    r5_check(None).unwrap();
}

#[test]
fn r5_ignoring_a_stop_during_setup_is_caught() {
    assert!(r5_check(Some(Mutant::IgnoreStopInSetup)).is_err());
}
