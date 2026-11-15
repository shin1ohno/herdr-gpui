//! R1, R6, R9, R12, R13 and the receipt's binding to its tunnel. Each rule's
//! check runs once as written (it must pass) and once with the rule's
//! mutant (it must fail), so CI shows that the test catches the removal.

use std::io::Read as _;

use super::*;
use crate::{
    hooks::Event,
    test_host::{Behavior, Seen, TestHost},
};

fn exit_zero() -> Behavior {
    Behavior::Exit {
        stdout: b"done".to_vec(),
        status: 0,
    }
}

/// R1: `Complete` comes only after russh's session task has ended and been
/// joined. The handler lives in that task, so at the receipt it must be
/// gone; the host must then see the session end.
fn r1_check(mutant: Option<Mutant>) -> Result<(), String> {
    let host = TestHost::start(Behavior::Flood);
    let mut run = start_watched(
        &target(host.port),
        &host.pins,
        watched_key(5, None),
        "flood",
        Deadlines::default(),
        removing(mutant),
    );
    // Output is flowing, so the session is past the exec request.
    let mut first = [0; 1];
    run.stream
        .read_exact(&mut first)
        .map_err(|e| e.to_string())?;
    run.tunnel.stop();
    let _ = receipt(&run.tunnel)?;
    let alive = run.probe.alive_at_settle().ok_or("no settle observed")?;
    if alive.handler != 0 {
        return Err(format!(
            "the session task still ran at the receipt: {alive:?}"
        ));
    }
    if !TestHost::wait_for(&host.seen.ended, 1, SETTLE) {
        return Err("the host never saw the session end".into());
    }
    Ok(())
}

#[test]
fn r1_receipt_follows_the_session_task_join() {
    r1_check(None).unwrap();
}

#[test]
fn r1_a_receipt_before_the_join_is_caught() {
    assert!(r1_check(Some(Mutant::ReceiptBeforeJoin)).is_err());
}

/// R6: a normal end (exit status and Close, no stop) also goes through the
/// teardown, and the exit is recorded before the receipt.
fn r6_check(mutant: Option<Mutant>) -> Result<(), String> {
    let host = TestHost::start(exit_zero());
    let mut run = start_watched(
        &target(host.port),
        &host.pins,
        watched_key(6, None),
        "exit",
        Deadlines::default(),
        removing(mutant),
    );
    let mut output = Vec::new();
    run.stream
        .read_to_end(&mut output)
        .map_err(|e| e.to_string())?;
    let _ = receipt(&run.tunnel)?;
    let exit = run.tunnel.wait_exit(Duration::ZERO);
    if exit
        != Some(TunnelExit::Remote {
            exit_status: Some(0),
            stderr_tail: Vec::new(),
        })
    {
        return Err(format!("unexpected exit {exit:?}"));
    }
    let events = run.probe.events();
    match (
        position(&events, Event::ExitRecorded),
        position(&events, Event::Receipt),
    ) {
        (Some(exit), Some(receipt)) if exit < receipt => Ok(()),
        _ => Err(format!(
            "the exit was not recorded before the receipt: {events:?}"
        )),
    }
}

#[test]
fn r6_a_normal_end_yields_a_receipt_after_the_exit() {
    r6_check(None).unwrap();
}

#[test]
fn r6_a_receipt_before_the_exit_is_caught() {
    assert!(r6_check(Some(Mutant::ReceiptBeforeExit)).is_err());
}

/// R9: every `Err` from `exec` comes before anything reaches the host.
fn r9_check(mutant: Option<Mutant>) -> Result<(), String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    for pins in [Pins::None, Pins::Keys(Vec::new())] {
        let started = start(
            &target(port),
            &software_key(1),
            &pins,
            "true",
            Deadlines::default(),
            removing(mutant),
        );
        if !matches!(started, Err(crate::Error::Failure(Failure::HostKeyUnknown))) {
            return Err(format!("unexpected result {:?}", started.map(|_| ())));
        }
    }
    // Long enough for a wrongly spawned task to have connected.
    let deadline = Instant::now() + Duration::from_millis(500);
    while Instant::now() < deadline {
        if listener.accept().is_ok() {
            return Err("a connection reached the host".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}

#[test]
fn r9_exec_errors_reach_no_host() {
    r9_check(None).unwrap();
}

#[test]
fn r9_connecting_before_the_pin_check_is_caught() {
    assert!(r9_check(Some(Mutant::ConnectBeforePinCheck)).is_err());
}

/// R12 and R13 (conditions 4 and 2): at the receipt the tunnel no longer
/// holds the client key, the pins (with the handler), or the task that
/// forwards the caller's input, which is still open here.
fn holders_check(mutant: Option<Mutant>) -> Result<(), String> {
    let host = TestHost::start(Behavior::Silent);
    let run = start_watched(
        &target(host.port),
        &host.pins,
        watched_key(12, None),
        "silent",
        Deadlines::default(),
        removing(mutant),
    );
    if !TestHost::wait_for(&host.seen.execs, 1, SETTLE) {
        return Err("the exec request never arrived".into());
    }
    run.tunnel.stop();
    let _ = receipt(&run.tunnel)?;
    let alive = run.probe.alive_at_settle().ok_or("no settle observed")?;
    if alive.key != 0 || alive.forwarder != 0 || alive.handler != 0 {
        return Err(format!("still held at the receipt: {alive:?}"));
    }
    // The caller's end was open all along.
    drop(run.stream);
    Ok(())
}

#[test]
fn r12_r13_the_receipt_follows_releasing_key_pins_and_input() {
    holders_check(None).unwrap();
}

#[test]
fn r12_keeping_the_key_and_pins_past_the_receipt_is_caught() {
    assert!(holders_check(Some(Mutant::KeepRequest)).is_err());
}

#[test]
fn r13_keeping_the_input_forwarder_past_the_receipt_is_caught() {
    assert!(holders_check(Some(Mutant::KeepInputForwarder)).is_err());
}

/// A receipt names its own tunnel, identities differ across hosts and
/// attempts, and every wait after the end returns the same outcome.
#[test]
fn a_receipt_is_bound_to_its_own_attempt() {
    let first = TestHost::start(exit_zero());
    let second = TestHost::start(exit_zero());
    let finish = |host: &TestHost, seed: u8| {
        let mut run = start_watched(
            &target(host.port),
            &host.pins,
            watched_key(seed, None),
            "exit",
            Deadlines::default(),
            Hooks::default(),
        );
        run.stream.read_to_end(&mut Vec::new()).unwrap();
        run
    };
    let a = finish(&first, 20);
    let b = finish(&second, 21);
    let again = finish(&first, 22);
    assert_ne!(a.tunnel.id(), b.tunnel.id());
    assert_ne!(a.tunnel.id(), again.tunnel.id());
    for run in [&a, &b, &again] {
        for _ in 0..3 {
            assert_eq!(receipt(&run.tunnel).unwrap().tunnel(), run.tunnel.id());
        }
    }
    assert_ne!(receipt(&a.tunnel).unwrap().tunnel(), b.tunnel.id());
    assert_eq!(Seen::get(&first.seen.connections), 2);
    assert_eq!(Seen::get(&second.seen.connections), 1);
}
