//! One carried connection must not hold up the others on the same forward:
//! a caller that stops reading, or a port that is slow to answer.

use std::{
    io::{Read as _, Write as _},
    net::{Shutdown, TcpListener},
    sync::mpsc,
    thread,
    time::Instant,
};

use super::{
    channels::{GREETING, Service, WAIT, forward, greeting},
    *,
};
use crate::test_host::{Behavior, TestHost};

/// A TCP service that writes to each connection until it goes away.
fn flood() -> Destination {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            thread::spawn(move || {
                let chunk = vec![b'x'; 64 * 1024];
                while stream.write_all(&chunk).is_ok() {}
            });
        }
    });
    Destination::localhost(port).unwrap()
}

/// Runs `open` on another thread and gives its result, or `None` when it
/// takes longer than `limit`.
fn within<T: Send + 'static>(
    limit: Duration,
    open: impl FnOnce() -> T + Send + 'static,
) -> Option<T> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(open());
    });
    receiver.recv_timeout(limit).ok()
}

#[test]
fn a_caller_that_stops_reading_does_not_stall_other_connections() {
    let host = TestHost::start(Behavior::Silent);
    let service = Service::start();
    let forward = Arc::new(forward(&host));
    // Opened and never read while the host floods it. The timeout is set
    // while the socket is still connected: once the relay drops it, macOS
    // refuses socket options on it (XNU `sosetopt` returns EINVAL for a
    // socket that can neither send nor receive), and Linux does not.
    let mut stalled = forward.open(flood()).unwrap();
    stalled.set_read_timeout(Some(WAIT)).unwrap();
    // Long enough for the flood to pass every buffer on the way, which
    // without the unread limit fills russh's channel queue and stops the
    // session task.
    thread::sleep(Duration::from_secs(2));
    let carried = Arc::clone(&forward);
    let destination = service.destination();
    let exchanged = within(WAIT, move || {
        let mut stream = carried.open(destination).unwrap();
        stream.set_read_timeout(Some(WAIT)).unwrap();
        let greeted = greeting(&mut stream);
        stream.write_all(b"still moving").unwrap();
        stream.shutdown(Shutdown::Write).unwrap();
        let mut echoed = Vec::new();
        stream.read_to_end(&mut echoed).unwrap();
        (greeted, echoed)
    });
    assert_eq!(
        exchanged,
        Some((GREETING.to_vec(), b"still moving".to_vec())),
        "a second connection must open and round-trip while the first is unread"
    );
    // The unread connection was reset once it fell too far behind: not
    // EOF, so a cut-off response cannot look whole.
    let ended = stalled.read_to_end(&mut Vec::new());
    assert!(
        matches!(&ended, Err(error) if error.kind() == io::ErrorKind::ConnectionReset),
        "the unread connection must be reset: {ended:?}"
    );
    assert!(
        forward.wait_end(Duration::ZERO).is_none(),
        "the forward itself carries on"
    );
}

#[test]
fn a_slow_open_delays_only_its_own_request() {
    let stalled_port = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let host = TestHost::start(Behavior::StallOpensTo(stalled_port));
    let service = Service::start();
    let forward = Arc::new(forward(&host));
    // Warm up: the connection is up before the slow open is queued.
    drop(forward.open(service.destination()).unwrap());
    let slow = {
        let forward = Arc::clone(&forward);
        thread::spawn(move || forward.open(Destination::localhost(stalled_port).unwrap()))
    };
    thread::sleep(Duration::from_millis(300));
    let started = Instant::now();
    let mut stream = forward.open(service.destination()).unwrap();
    let waited = started.elapsed();
    stream.set_read_timeout(Some(WAIT)).unwrap();
    assert_eq!(greeting(&mut stream), GREETING);
    assert!(
        waited < Duration::from_secs(5),
        "an open behind a stalled one took {waited:?}"
    );
    forward.stop();
    assert!(matches!(slow.join().unwrap(), Err(Error::Stopped)));
}

/// A TCP service that writes `bytes` to each connection and then closes it.
fn fixed(bytes: Vec<u8>) -> Destination {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let bytes = Arc::new(bytes);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let bytes = Arc::clone(&bytes);
            thread::spawn(move || {
                let _ = stream.write_all(&bytes);
            });
        }
    });
    Destination::localhost(port).unwrap()
}

#[test]
fn a_slow_reader_within_the_limit_gets_every_byte() {
    let host = TestHost::start(Behavior::Silent);
    let forward = forward(&host);
    // Below the limit, so the relay keeps it all while the reader lags and
    // writes to the caller's socket go through in pieces.
    let sent: Vec<u8> = (0..1_500_000u32).map(|i| (i % 251) as u8).collect();
    let mut stream = forward.open(fixed(sent.clone())).unwrap();
    stream.set_read_timeout(Some(WAIT)).unwrap();
    let mut received = Vec::new();
    let mut chunk = [0; 8192];
    loop {
        let read = stream.read(&mut chunk).unwrap();
        if read == 0 {
            break;
        }
        received.extend_from_slice(&chunk[..read]);
        thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(received.len(), sent.len());
    assert!(received == sent, "the bytes arrive whole and in order");
}
