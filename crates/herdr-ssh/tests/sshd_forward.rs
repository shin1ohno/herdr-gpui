//! Explicitly opt-in: forwards through the selected OpenSSH `sshd`, started
//! in a private sandbox on its own port with its own host key and
//! `authorized_keys`, to a TCP service this test runs on the loopback.
#![cfg(all(unix, not(target_os = "ios")))]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_methods)]

#[allow(dead_code)]
#[path = "../../test-support/sandbox.rs"]
mod sandbox;
#[path = "../../test-support/sshd.rs"]
mod sshd;

use anyhow::{Result, bail, ensure};
use herdr_ssh::{
    ClientKey, Forward, KnownHosts, Pins, Scan, Target,
    forward::{Destination, Error},
};
use p256::ecdsa::{Signature, SigningKey, signature::Signer as _};
use sshd::Sshd;
use std::{
    io::{Read as _, Write as _},
    net::{Shutdown, TcpListener, TcpStream},
    os::unix::net::UnixStream,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};

const DEADLINE: Duration = Duration::from_secs(15);
const GREETING: &[u8] = b"hello from the host's port\n";

fn software_key(seed: u8) -> ClientKey {
    let signing = SigningKey::from_slice(&[seed; 32]).unwrap();
    let public = signing
        .verifying_key()
        .to_sec1_point(false)
        .as_bytes()
        .to_vec();
    ClientKey::new(public, move |message| {
        let signature: Signature = signing.sign(message);
        Ok(signature.to_bytes().into())
    })
    .unwrap()
}

/// The key's `authorized_keys` line with `options` in place of `restrict`.
fn line_with(key: &ClientKey, options: &str) -> String {
    let line = key.authorized_keys_line("herdr-test");
    let rest = line.strip_prefix("restrict ").unwrap();
    if options.is_empty() {
        rest.to_owned()
    } else {
        format!("{options} {rest}")
    }
}

/// A TCP service on the loopback: each connection gets the greeting, then
/// its own bytes back until it sends EOF.
struct Service {
    port: u16,
    accepted: Arc<AtomicUsize>,
}

impl Service {
    fn start() -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        let accepted = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&accepted);
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { return };
                counter.fetch_add(1, Ordering::SeqCst);
                thread::spawn(move || {
                    let _ = stream.write_all(GREETING);
                    let mut echo = stream.try_clone().unwrap();
                    let _ = std::io::copy(&mut stream, &mut echo);
                    let _ = echo.shutdown(Shutdown::Write);
                });
            }
        });
        Ok(Self { port, accepted })
    }

    fn destination(&self) -> Destination {
        Destination::localhost(self.port).unwrap()
    }
}

/// A port that nothing listens on.
fn closed_port() -> Result<u16> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}

/// Pins the private host's key through a scan, as the app's trust flow does.
fn trusted(server: &Sshd, port: u16) -> Result<(Target, Pins)> {
    let target = Target::parse(&format!(
        "{}@127.0.0.1:{port}",
        server.target()?.split('@').next().unwrap()
    ))?;
    let Scan::New(keys) = herdr_ssh::scan(&target, &Pins::None, &AtomicBool::new(false))? else {
        bail!("a new private host must require trust");
    };
    let hosts = KnownHosts::new(server.directory().join(format!("known_hosts-{port}")));
    hosts.trust(&target, &keys)?;
    let pins = hosts.pins(&target)?;
    Ok((target, pins))
}

/// Writes `payload` and EOF from another thread while this one reads until
/// EOF.
fn exchange(stream: &UnixStream, payload: Vec<u8>) -> Result<Vec<u8>> {
    let mut writer = stream.try_clone()?;
    let writing = thread::spawn(move || -> std::io::Result<()> {
        writer.write_all(&payload)?;
        writer.shutdown(Shutdown::Write)
    });
    let mut output = Vec::new();
    (&*stream).read_to_end(&mut output)?;
    writing.join().unwrap()?;
    Ok(output)
}

#[test]
#[ignore = "requires explicit HERDR_TEST_SSHD and HERDR_TEST_SSH_KEYGEN; starts a private sshd"]
fn private_sshd_forward_carries_bytes_both_ways() -> Result<()> {
    let key = software_key(21);
    let service = Service::start()?;
    let server = Sshd::start(&line_with(&key, ""), "ed25519")?;
    let (target, pins) = trusted(&server, server.port())?;
    let forward = Forward::connect(&target, &key, &pins)?;

    let stream = forward.open(service.destination())?;
    stream.set_read_timeout(Some(DEADLINE))?;
    let payload: Vec<u8> = (0..=255).cycle().take(300_000).collect();
    let output = exchange(&stream, payload.clone())?;
    ensure!(
        output.len() == GREETING.len() + payload.len(),
        "{} bytes back",
        output.len()
    );
    let (greeting, echoed) = output.split_at(GREETING.len());
    ensure!(greeting == GREETING, "the port's bytes reach the caller");
    ensure!(
        echoed == payload,
        "the caller's bytes reach the port and come back"
    );

    // A connection accepted on a local listener, carried as it is.
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let mut client = TcpStream::connect(listener.local_addr()?)?;
    let (accepted, _) = listener.accept()?;
    forward.relay(service.destination(), accepted)?;
    client.set_read_timeout(Some(DEADLINE))?;
    client.write_all(b"ping")?;
    client.shutdown(Shutdown::Write)?;
    let mut output = Vec::new();
    client.read_to_end(&mut output)?;
    ensure!(output == [GREETING, b"ping"].concat());
    ensure!(service.accepted.load(Ordering::SeqCst) == 2);
    ensure!(forward.wait_end(Duration::ZERO).is_none());
    Ok(())
}

#[test]
#[ignore = "requires explicit HERDR_TEST_SSHD and HERDR_TEST_SSH_KEYGEN; starts a private sshd"]
fn private_sshd_permitopen_allows_only_its_port() -> Result<()> {
    let key = software_key(22);
    let service = Service::start()?;
    let other = Service::start()?;
    let options = format!(
        "restrict,port-forwarding,permitopen=\"localhost:{}\"",
        service.port
    );
    let server = Sshd::start(&line_with(&key, &options), "ed25519")?;
    let (target, pins) = trusted(&server, server.port())?;
    let forward = Forward::connect(&target, &key, &pins)?;
    let stream = forward.open(service.destination())?;
    stream.set_read_timeout(Some(DEADLINE))?;
    ensure!(exchange(&stream, b"allowed".to_vec())? == [GREETING, b"allowed"].concat());
    let refused = forward.open(other.destination());
    ensure!(matches!(refused, Err(Error::Prohibited)), "{refused:?}");
    ensure!(other.accepted.load(Ordering::SeqCst) == 0);
    Ok(())
}

#[test]
#[ignore = "requires explicit HERDR_TEST_SSHD and HERDR_TEST_SSH_KEYGEN; starts a private sshd"]
fn private_sshd_restrict_key_is_prohibited() -> Result<()> {
    let key = software_key(23);
    let service = Service::start()?;
    // The line `ClientKey::authorized_keys_line` gives, `restrict` included.
    let server = Sshd::start(&key.authorized_keys_line("herdr-test"), "ed25519")?;
    let (target, pins) = trusted(&server, server.port())?;
    let forward = Forward::connect(&target, &key, &pins)?;
    for _ in 0..2 {
        let refused = forward.open(service.destination());
        ensure!(matches!(refused, Err(Error::Prohibited)), "{refused:?}");
    }
    // A refused channel is not a lost connection.
    ensure!(forward.wait_end(Duration::ZERO).is_none());
    ensure!(service.accepted.load(Ordering::SeqCst) == 0);
    Ok(())
}

#[test]
#[ignore = "requires explicit HERDR_TEST_SSHD and HERDR_TEST_SSH_KEYGEN; starts a private sshd"]
fn private_sshd_without_tcp_forwarding_is_prohibited() -> Result<()> {
    let key = software_key(24);
    let service = Service::start()?;
    let server = Sshd::start_with(&line_with(&key, ""), "ed25519", "AllowTcpForwarding no\n")?;
    let (target, pins) = trusted(&server, server.port())?;
    let forward = Forward::connect(&target, &key, &pins)?;
    let refused = forward.open(service.destination());
    ensure!(matches!(refused, Err(Error::Prohibited)), "{refused:?}");
    Ok(())
}

#[test]
#[ignore = "requires explicit HERDR_TEST_SSHD and HERDR_TEST_SSH_KEYGEN; starts a private sshd"]
fn private_sshd_closed_port_is_typed() -> Result<()> {
    let key = software_key(25);
    let server = Sshd::start(&line_with(&key, ""), "ed25519")?;
    let (target, pins) = trusted(&server, server.port())?;
    let forward = Forward::connect(&target, &key, &pins)?;
    let refused = forward.open(Destination::localhost(closed_port()?)?);
    ensure!(matches!(refused, Err(Error::PortClosed)), "{refused:?}");
    // The connection carries on: an open port still works.
    let service = Service::start()?;
    let stream = forward.open(service.destination())?;
    stream.set_read_timeout(Some(DEADLINE))?;
    ensure!(exchange(&stream, Vec::new())? == GREETING);
    Ok(())
}

/// A TCP relay between the client and the private sshd that the test can
/// cut, as a network that goes away.
struct Cuttable {
    port: u16,
    open: Arc<Mutex<Vec<TcpStream>>>,
}

impl Cuttable {
    fn start(to: u16) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        let open = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&open);
        thread::spawn(move || {
            for client in listener.incoming() {
                let Ok(client) = client else { return };
                let Ok(server) = TcpStream::connect(("127.0.0.1", to)) else {
                    return;
                };
                {
                    let mut kept = kept.lock().unwrap();
                    kept.push(client.try_clone().unwrap());
                    kept.push(server.try_clone().unwrap());
                }
                for (mut from, mut to) in [
                    (client.try_clone().unwrap(), server.try_clone().unwrap()),
                    (server, client),
                ] {
                    thread::spawn(move || {
                        let _ = std::io::copy(&mut from, &mut to);
                        let _ = to.shutdown(Shutdown::Write);
                    });
                }
            }
        });
        Ok(Self { port, open })
    }

    fn cut(&self) {
        for stream in self.open.lock().unwrap().drain(..) {
            let _ = stream.shutdown(Shutdown::Both);
        }
    }
}

#[test]
#[ignore = "requires explicit HERDR_TEST_SSHD and HERDR_TEST_SSH_KEYGEN; starts a private sshd"]
fn private_sshd_lost_connection_ends_the_channel() -> Result<()> {
    let key = software_key(26);
    let service = Service::start()?;
    let server = Sshd::start(&line_with(&key, ""), "ed25519")?;
    let relay = Cuttable::start(server.port())?;
    let (target, pins) = trusted(&server, relay.port)?;
    let forward = Forward::connect(&target, &key, &pins)?;
    let mut stream = forward.open(service.destination())?;
    stream.set_read_timeout(Some(DEADLINE))?;
    let mut greeting = vec![0; GREETING.len()];
    stream.read_exact(&mut greeting)?;
    ensure!(greeting == GREETING);

    relay.cut();
    // The carried connection ends: EOF or a reset, never a hang.
    let mut rest = Vec::new();
    match stream.read_to_end(&mut rest) {
        Ok(_) => ensure!(rest.is_empty()),
        Err(error) => ensure!(
            error.kind() != std::io::ErrorKind::WouldBlock
                && error.kind() != std::io::ErrorKind::TimedOut,
            "the channel did not end: {error}"
        ),
    }
    let end = forward.wait_end(DEADLINE);
    ensure!(matches!(end, Some(Error::ConnectionLost(_))), "{end:?}");
    let after = forward.open(service.destination());
    ensure!(matches!(after, Err(Error::ConnectionLost(_))), "{after:?}");
    Ok(())
}
