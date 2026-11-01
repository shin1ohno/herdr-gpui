use super::*;

#[test]
fn an_ipad_rejects_a_local_daemon_before_resolving_or_starting_it() {
    let started = std::cell::Cell::new(false);
    let result = connect(&ConnectTarget::Local, &AtomicBool::new(false), || {
        started.set(true);
    });
    let error = result.err().expect("iPad cannot connect to a local daemon");
    assert_eq!(error.kind(), io::ErrorKind::Unsupported);
    assert!(error.get_ref().is_some_and(|source| {
        matches!(source.downcast_ref::<Error>(), Some(Error::Unavailable))
    }));
    assert!(!started.get());
}
