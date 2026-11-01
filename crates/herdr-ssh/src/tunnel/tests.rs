#![allow(clippy::unwrap_used)]

use std::{
    io::Read as _,
    net::TcpListener,
    time::{Duration, Instant},
};

use super::*;
use crate::client_key::tests::software_key;
use crate::host_key::tests::ED25519;

fn pins() -> Pins {
    Pins::Keys(vec![
        HostKey::from_known_hosts("ssh-ed25519", ED25519).unwrap(),
    ])
}

/// A loopback port that was free a moment ago and has nothing listening.
fn closed_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn target(port: u16) -> Target {
    Target::parse(&format!("someone@127.0.0.1:{port}")).unwrap()
}

#[test]
fn exec_without_pins_connects_nowhere() {
    for pins in [Pins::None, Pins::Keys(Vec::new())] {
        assert!(matches!(
            exec(&target(22), &software_key(1), &pins, "true"),
            Err(crate::Error::Failure(Failure::HostKeyUnknown))
        ));
    }
}

#[test]
fn exec_reports_failure_after_ending_the_stream() {
    let (mut stream, tunnel) =
        exec(&target(closed_port()), &software_key(1), &pins(), "true").unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .unwrap();
    let mut rest = Vec::new();
    assert_eq!(stream.read_to_end(&mut rest).unwrap(), 0);
    // The exit is recorded before the stream closes, so no wait is needed.
    assert_eq!(
        tunnel.wait_exit(Duration::ZERO),
        Some(TunnelExit::Ssh(Failure::Unreachable))
    );
}

#[test]
fn scan_observes_a_stop_set_before_it_starts() {
    let stop = AtomicBool::new(true);
    let started = Instant::now();
    assert!(matches!(
        scan(&target(closed_port()), &Pins::None, &stop),
        Err(crate::Error::Failure(Failure::Cancelled))
    ));
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn scan_of_a_closed_port_is_unreachable() {
    let stop = AtomicBool::new(false);
    assert!(matches!(
        scan(&target(closed_port()), &pins(), &stop),
        Err(crate::Error::Failure(Failure::Unreachable))
    ));
}

#[test]
fn wait_exit_times_out_until_set() {
    let slot = ExitSlot::default();
    let started = Instant::now();
    assert_eq!(slot.wait(Duration::from_millis(50)), None);
    assert!(started.elapsed() >= Duration::from_millis(50));
    slot.set(TunnelExit::Ssh(Failure::Auth));
    assert_eq!(
        slot.wait(Duration::ZERO),
        Some(TunnelExit::Ssh(Failure::Auth))
    );
}

#[test]
fn dropping_a_tunnel_does_not_block() {
    // A listener that accepts and never speaks keeps the tunnel connecting.
    let silent = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = silent.local_addr().unwrap().port();
    let (stream, tunnel) = exec(&target(port), &software_key(1), &pins(), "true").unwrap();
    let started = Instant::now();
    drop(tunnel);
    assert!(started.elapsed() < Duration::from_millis(100));
    // Cancelling drops the runtime's end, so the caller reads end-of-stream.
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut rest = Vec::new();
    assert_eq!((&stream).read_to_end(&mut rest).unwrap(), 0);
}
