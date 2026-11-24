use super::*;

/// A listener's connection that reaches its queue after the first accept,
/// as Darwin's loopback delivers one under load, is still the one paired.
#[test]
fn a_connection_queued_late_is_still_accepted() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let late = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        TcpStream::connect(address).unwrap()
    });
    let accepted = accept_from(&listener, |_| true, PAIR_PATIENCE).unwrap();
    let caller = late.join().unwrap();
    assert_eq!(accepted.peer_addr().unwrap(), caller.local_addr().unwrap());
}

/// Nothing connecting is a bounded wait, then the accept's own error.
#[test]
fn nothing_queued_fails_once_patience_runs_out() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let started = Instant::now();
    let error = accept_from(&listener, |_| true, Duration::from_millis(30)).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::WouldBlock);
    assert!(started.elapsed() >= Duration::from_millis(30));
    assert!(started.elapsed() < Duration::from_secs(5));
}

/// Another local process that connects first is closed, never paired.
#[test]
fn other_connections_are_closed_and_not_paired() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let stranger = TcpStream::connect(address).unwrap();
    let own = TcpStream::connect(address).unwrap();
    let origin = own.local_addr().unwrap();
    let accepted = accept_from(&listener, |peer| peer == origin, PAIR_PATIENCE).unwrap();
    assert_eq!(accepted.peer_addr().unwrap(), origin);
    let mut byte = [0; 1];
    stranger
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    assert_eq!(io::Read::read(&mut &stranger, &mut byte).unwrap(), 0);
}

/// More strangers than the pair accepts end the search.
#[test]
fn too_many_strangers_end_the_search() {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let strangers: Vec<_> = (0..PAIR_ACCEPTS)
        .map(|_| TcpStream::connect(address).unwrap())
        .collect();
    let error = accept_from(&listener, |_| false, PAIR_PATIENCE).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::AddrInUse);
    drop(strangers);
}
