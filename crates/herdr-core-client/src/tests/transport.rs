use super::*;
use crate::{Core, Error, hello::Mismatch};
use std::sync::mpsc::{self, Sender};

struct Guarded {
    socket: Socket,
    dropped: Sender<()>,
}

impl Read for Guarded {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        self.socket.read(bytes)
    }
}

impl Write for Guarded {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.socket.write(bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.socket.flush()
    }
}

impl Drop for Guarded {
    fn drop(&mut self) {
        let _ = self.dropped.send(());
    }
}

#[test]
fn a_supplied_transport_keeps_its_guard_until_the_worker_ends() {
    let (client, server) = pair();
    let (dropped, observed) = mpsc::channel();
    let peer = std::thread::spawn(move || {
        MockCore::accept(server, &hello_reply("0.1.13", &every_method()))
    });
    let client = Core::over(Guarded {
        socket: client,
        dropped,
    })
    .unwrap();
    let (mut peer, _) = peer.join().unwrap();
    assert!(observed.try_recv().is_err());
    let read = std::thread::spawn(move || {
        let terminals = client.list(false).unwrap();
        (client, terminals)
    });
    let request = peer.request();
    peer.answer(&request, json!({"terminals": [], "hidden_finished": 0}));
    let (client, terminals) = read.join().unwrap();
    assert!(terminals.terminals.is_empty());
    assert!(observed.try_recv().is_err());
    drop(client);
    observed.recv_timeout(Duration::from_secs(5)).unwrap();
}

#[test]
fn a_rejected_handshake_releases_the_supplied_transport() {
    let (client, server) = pair();
    let (dropped, observed) = mpsc::channel();
    let peer = std::thread::spawn(move || {
        MockCore::accept(server, &hello_reply("0.1.0", &every_method()))
    });
    assert!(matches!(
        Core::over(Guarded {
            socket: client,
            dropped
        }),
        Err(Error::Compat(Mismatch::TooOld { .. }))
    ));
    observed.recv_timeout(Duration::from_secs(5)).unwrap();
    drop(peer.join().unwrap());
}

#[test]
fn a_terminated_hello_still_obeys_the_size_limit() {
    let (client, server) = pair();
    let mut reply = hello_reply("0.1.13", &every_method());
    reply["result"]["padding"] = json!("");
    let overhead = serde_json::to_vec(&reply).unwrap().len();
    reply["result"]["padding"] = json!("x".repeat((64 << 10) + 10 - overhead));
    let peer = std::thread::spawn(move || MockCore::accept(server, &reply));
    assert!(matches!(
        Core::over(client),
        Err(Error::Protocol(crate::Violation::LongHello(_)))
    ));
    drop(peer.join().unwrap());
}

#[test]
fn explicit_disconnect_closes_a_transport_even_while_the_core_is_retained() {
    let (client, server) = pair();
    let (dropped, observed) = mpsc::channel();
    let peer = std::thread::spawn(move || {
        MockCore::accept(server, &hello_reply("0.1.13", &every_method()))
    });
    let client = Core::over(Guarded {
        socket: client,
        dropped,
    })
    .unwrap();
    let peer = peer.join().unwrap();
    client.disconnect();
    observed.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(client.is_lost());
    assert!(matches!(client.list(false), Err(Error::Lost)));
    drop(peer);
}
