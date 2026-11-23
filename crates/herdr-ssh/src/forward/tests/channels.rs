//! Forwards against the in-process test host, which relays `direct-tcpip`
//! channels to this machine's loopback as sshd does. The private-sshd
//! tests (`tests/sshd_forward.rs`) cover OpenSSH itself.

use std::{
    io::{Read as _, Write as _},
    net::{Shutdown, TcpListener},
    sync::atomic::AtomicUsize,
    thread,
};

use super::*;
use crate::{
    HostKey,
    client_key::tests::software_key,
    host_key::tests::ED25519,
    test_host::{Behavior, Seen, TestHost},
};

pub(super) const WAIT: Duration = Duration::from_secs(10);
pub(super) const GREETING: &[u8] = b"hello from the host's port\n";

/// A TCP service on this machine's loopback: each connection gets the
/// greeting, then its own bytes back until it sends EOF.
pub(super) struct Service {
    port: u16,
    accepted: Arc<AtomicUsize>,
}

impl Service {
    pub(super) fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let accepted = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&accepted);
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                thread::spawn(move || {
                    let _ = stream.write_all(GREETING);
                    let mut echo = stream.try_clone().unwrap();
                    let _ = std::io::copy(&mut stream, &mut echo);
                    let _ = echo.shutdown(Shutdown::Write);
                });
            }
        });
        Self { port, accepted }
    }

    pub(super) fn destination(&self) -> Destination {
        Destination::localhost(self.port).unwrap()
    }
}

pub(super) fn forward(host: &TestHost) -> Forward {
    let target = Target::parse(&format!("user@127.0.0.1:{}", host.port)).unwrap();
    Forward::connect(&target, &software_key(42), &host.pins).unwrap()
}

/// Reads the service's greeting.
pub(super) fn greeting(stream: &mut impl std::io::Read) -> Vec<u8> {
    let mut greeting = vec![0; GREETING.len()];
    stream.read_exact(&mut greeting).unwrap();
    greeting
}

/// Writes `payload` and EOF from another thread while this one reads
/// everything until EOF, so neither direction waits for the other.
fn exchange(stream: &UnixStream, payload: Vec<u8>) -> Vec<u8> {
    let mut writer = stream.try_clone().unwrap();
    let writing = thread::spawn(move || {
        writer.write_all(&payload).unwrap();
        writer.shutdown(Shutdown::Write).unwrap();
    });
    let mut output = Vec::new();
    (&*stream).read_to_end(&mut output).unwrap();
    writing.join().unwrap();
    output
}

#[test]
fn bytes_cross_both_ways_over_an_opened_channel() {
    let host = TestHost::start(Behavior::Silent);
    let service = Service::start();
    let forward = forward(&host);
    let stream = forward.open(service.destination()).unwrap();
    stream.set_read_timeout(Some(WAIT)).unwrap();
    let payload: Vec<u8> = (0..=255).cycle().take(200_000).collect();
    let output = exchange(&stream, payload.clone());
    let (greeting, echoed) = output.split_at(GREETING.len().min(output.len()));
    assert_eq!(greeting, GREETING, "the host's port reaches the caller");
    assert_eq!(
        echoed, payload,
        "the caller's bytes reach the port and back"
    );
    assert_eq!(Seen::get(&host.seen.direct_tcpip), 1);
    // A forward opens no session and runs nothing.
    assert_eq!(Seen::get(&host.seen.sessions), 0);
    assert_eq!(Seen::get(&host.seen.execs), 0);
}

#[test]
fn a_relayed_tcp_connection_reaches_the_port() {
    let host = TestHost::start(Behavior::Silent);
    let service = Service::start();
    let forward = forward(&host);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (accepted, _) = listener.accept().unwrap();
    forward.relay(service.destination(), accepted).unwrap();
    client.set_read_timeout(Some(WAIT)).unwrap();
    assert_eq!(greeting(&mut client), GREETING);
    client.write_all(b"ping").unwrap();
    client.shutdown(Shutdown::Write).unwrap();
    let mut echoed = Vec::new();
    client.read_to_end(&mut echoed).unwrap();
    assert_eq!(echoed, b"ping");
}

#[test]
fn channels_share_one_connection() {
    let host = TestHost::start(Behavior::Silent);
    let service = Service::start();
    let forward = forward(&host);
    let streams: Vec<_> = (0..3)
        .map(|_| forward.open(service.destination()).unwrap())
        .collect();
    assert!(TestHost::wait_for(&service.accepted, 3, WAIT));
    assert_eq!(Seen::get(&host.seen.connections), 1);
    assert_eq!(Seen::get(&host.seen.auths), 1);
    drop(streams);
}

#[test]
fn a_prohibited_channel_is_typed_and_keeps_the_connection() {
    let host = TestHost::start(Behavior::ProhibitForwarding);
    let service = Service::start();
    let forward = forward(&host);
    for _ in 0..2 {
        assert!(matches!(
            forward.open(service.destination()),
            Err(Error::Prohibited)
        ));
    }
    assert_eq!(Seen::get(&host.seen.connections), 1);
    assert!(forward.wait_end(Duration::ZERO).is_none());
    assert_eq!(Seen::get(&service.accepted), 0);
}

#[test]
fn a_closed_port_is_typed_and_keeps_the_connection() {
    let host = TestHost::start(Behavior::Silent);
    let closed = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let forward = forward(&host);
    assert!(matches!(
        forward.open(Destination::localhost(closed).unwrap()),
        Err(Error::PortClosed)
    ));
    let service = Service::start();
    let mut stream = forward.open(service.destination()).unwrap();
    stream.set_read_timeout(Some(WAIT)).unwrap();
    assert_eq!(greeting(&mut stream), GREETING);
    assert_eq!(Seen::get(&host.seen.connections), 1);
}

#[test]
fn a_lost_connection_ends_its_channels_and_is_typed() {
    let host = TestHost::start(Behavior::Silent);
    let service = Service::start();
    let forward = forward(&host);
    let mut stream = forward.open(service.destination()).unwrap();
    stream.set_read_timeout(Some(WAIT)).unwrap();
    assert_eq!(greeting(&mut stream), GREETING);
    // Ends the host's runtime, and with it every connection it served.
    drop(host);
    let mut rest = Vec::new();
    // EOF or a reset: either way the carried connection is over.
    let _ = stream.read_to_end(&mut rest);
    assert!(rest.is_empty());
    assert!(matches!(
        forward.wait_end(WAIT),
        Some(Error::ConnectionLost(_))
    ));
    assert!(matches!(
        forward.open(service.destination()),
        Err(Error::ConnectionLost(_))
    ));
    assert!(matches!(
        forward.wait_teardown(WAIT),
        Some(Teardown::Complete(_))
    ));
}

#[test]
fn the_client_asks_for_no_other_forwarding_and_refuses_channels_back() {
    let host = TestHost::start(Behavior::OpensChannelsBack);
    let service = Service::start();
    let forward = forward(&host);
    let stream = forward.open(service.destination()).unwrap();
    // A forwarded-tcpip, an X11, an agent, and a session channel.
    assert!(TestHost::wait_for(&host.seen.refused_by_client, 4, WAIT));
    assert_eq!(Seen::get(&host.seen.forwarding_requests), 0);
    assert_eq!(Seen::get(&host.seen.sessions), 0);
    drop(stream);
}

#[test]
fn a_stop_ends_carried_connections_and_gives_a_receipt() {
    let host = TestHost::start(Behavior::Silent);
    let service = Service::start();
    let forward = forward(&host);
    let mut stream = forward.open(service.destination()).unwrap();
    stream.set_read_timeout(Some(WAIT)).unwrap();
    assert_eq!(greeting(&mut stream), GREETING);
    forward.stop();
    assert!(matches!(
        forward.wait_teardown(WAIT),
        Some(Teardown::Complete(_))
    ));
    let _ = stream.read_to_end(&mut Vec::new());
    assert!(matches!(forward.wait_end(WAIT), Some(Error::Stopped)));
    assert!(matches!(
        forward.open(service.destination()),
        Err(Error::Stopped)
    ));
    assert!(TestHost::wait_for(&host.seen.ended, 1, WAIT));
}

#[test]
fn a_changed_host_key_fails_the_connection() {
    let host = TestHost::start(Behavior::Silent);
    let service = Service::start();
    let target = Target::parse(&format!("user@127.0.0.1:{}", host.port)).unwrap();
    let other = Pins::Keys(vec![
        HostKey::from_known_hosts("ssh-ed25519", ED25519).unwrap(),
    ]);
    let forward = Forward::connect(&target, &software_key(42), &other).unwrap();
    assert!(matches!(
        forward.open(service.destination()),
        Err(Error::Connect(Failure::HostKeyChanged))
    ));
    assert_eq!(
        Seen::get(&host.seen.auths),
        0,
        "no key is offered to a changed host"
    );
}

#[test]
fn connections_beyond_the_limit_are_busy() {
    let host = TestHost::start(Behavior::Silent);
    let service = Service::start();
    let forward = forward(&host);
    let streams: Vec<_> = (0..MAX_CHANNELS)
        .map(|_| forward.open(service.destination()).unwrap())
        .collect();
    assert!(matches!(
        forward.open(service.destination()),
        Err(Error::Busy)
    ));
    drop(streams);
}

#[test]
fn dropping_the_forward_ends_its_connections() {
    let host = TestHost::start(Behavior::Silent);
    let service = Service::start();
    let forward = forward(&host);
    let mut stream = forward.open(service.destination()).unwrap();
    stream.set_read_timeout(Some(WAIT)).unwrap();
    assert_eq!(greeting(&mut stream), GREETING);
    drop(forward);
    // EOF or a reset within the wait, never a hang.
    let mut rest = Vec::new();
    if let Err(error) = stream.read_to_end(&mut rest) {
        assert_ne!(error.kind(), std::io::ErrorKind::WouldBlock, "{error}");
    }
    assert!(rest.is_empty());
    assert!(TestHost::wait_for(&host.seen.ended, 1, WAIT));
}
