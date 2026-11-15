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

// Teardown receipts (plan rules R1-R13); fixtures shared by the topic files.

/// How long a test waits for a teardown that should settle.
const SETTLE: Duration = Duration::from_secs(15);

/// Holds a signing closure until opened.
#[derive(Default)]
struct SignGate {
    open: Mutex<bool>,
    changed: Condvar,
    entered: AtomicBool,
}

impl SignGate {
    fn pass(&self) {
        self.entered.store(true, Ordering::SeqCst);
        let open = self.open.lock().unwrap();
        let _open = self.changed.wait_while(open, |open| !*open).unwrap();
    }

    fn open(&self) {
        *self.open.lock().unwrap() = true;
        self.changed.notify_all();
    }

    fn wait_entered(&self, limit: Duration) -> bool {
        let started = Instant::now();
        while !self.entered.load(Ordering::SeqCst) {
            if started.elapsed() > limit {
                return false;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        true
    }
}

/// A software key whose signing closure alone holds the returned stand-in,
/// and which first waits at `gate`, if any.
fn watched_key(seed: u8, gate: Option<Arc<SignGate>>) -> (ClientKey, std::sync::Weak<()>) {
    use p256::ecdsa::{Signature, SigningKey, signature::Signer as _};
    let signing = SigningKey::from_slice(&[seed; 32]).unwrap();
    let public = signing
        .verifying_key()
        .to_sec1_point(false)
        .as_bytes()
        .to_vec();
    let stand_in = Arc::new(());
    let watched = Arc::downgrade(&stand_in);
    let key = ClientKey::new(public, move |message| {
        let _held = &stand_in;
        if let Some(gate) = &gate {
            gate.pass();
        }
        let signature: Signature = signing.sign(message);
        Ok(signature.to_bytes().into())
    })
    .unwrap();
    (key, watched)
}

fn removing(mutant: Option<Mutant>) -> Hooks {
    Hooks {
        mutant,
        ..Hooks::default()
    }
}

/// A started tunnel with its probe. The caller's stream stays open until
/// this is dropped.
struct Watched {
    stream: UnixStream,
    tunnel: Tunnel,
    probe: Arc<crate::hooks::Probe>,
}

/// Starts a tunnel to `target` and drops the test's copy of `key`, so only
/// the tunnel holds the key's stand-in.
fn start_watched(
    target: &Target,
    pins: &Pins,
    (key, watched): (ClientKey, std::sync::Weak<()>),
    command: &str,
    deadlines: Deadlines,
    mut hooks: Hooks,
) -> Watched {
    let probe = Arc::new(crate::hooks::Probe::default());
    probe.watch_key(watched);
    hooks.probe = Some(Arc::clone(&probe));
    let (stream, tunnel) = start(target, &key, pins, command, deadlines, hooks).unwrap();
    drop(key);
    stream.set_read_timeout(Some(SETTLE)).unwrap();
    Watched {
        stream,
        tunnel,
        probe,
    }
}

/// The receipt, if the teardown settles `Complete` within [`SETTLE`].
fn receipt(tunnel: &Tunnel) -> Result<crate::TeardownReceipt, String> {
    match tunnel.wait_teardown(SETTLE) {
        Some(Teardown::Complete(receipt)) => Ok(receipt),
        Some(Teardown::Unproven) => Err("the teardown settled unproven".into()),
        None => Err("the teardown did not settle".into()),
    }
}

/// Where `event` happened among the probe's events.
fn position(events: &[crate::hooks::Event], event: crate::hooks::Event) -> Option<usize> {
    events.iter().position(|seen| *seen == event)
}

mod faults;
mod logs;
mod open_stage;
mod receipt;
mod signing;
