use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

static ENTERED: AtomicBool = AtomicBool::new(false);
static FINISHED: AtomicBool = AtomicBool::new(false);
static GATE: (Mutex<bool>, Condvar) = (Mutex::new(false), Condvar::new());

fn join(target: &ConnectTarget, _: Start) -> Result<CoreBackend, Error> {
    if *target == ConnectTarget::Local {
        ENTERED.store(true, Ordering::Release);
        let (open, wake) = &GATE;
        let guard = open.lock().unwrap();
        let (open, waited) = wake
            .wait_timeout_while(guard, Duration::from_secs(5), |open| !*open)
            .unwrap();
        assert!(*open && !waited.timed_out());
        FINISHED.store(true, Ordering::Release);
        return Err(Error::Namespace);
    }
    Err(Error::Unavailable)
}

fn wait_for(flag: &AtomicBool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !flag.load(Ordering::Acquire) {
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
}

#[test]
fn retargeting_fences_the_old_attempt_and_includes_the_session() {
    let mut hub = Hub::with_join(join);
    hub.connect(Start::Join);
    wait_for(&ENTERED);
    let first = ConnectTarget::Ssh {
        target: "user@box".into(),
        session: "first".into(),
    };
    assert!(hub.retarget(&first));
    hub.connect(Start::Join);
    assert!(matches!(hub.state(), State::Idle));
    hub.connect(Start::IfMissing);
    settle(&mut hub);
    {
        let (open, wake) = &GATE;
        *open.lock().unwrap() = true;
        wake.notify_all();
    }
    wait_for(&FINISHED);
    hub.poll();
    assert!(
        matches!(hub.state(), State::Unavailable(error) if matches!(**error, Error::Unavailable))
    );
    assert!(!hub.retarget(&first));
    let second = ConnectTarget::Ssh {
        target: "user@box".into(),
        session: "second".into(),
    };
    assert!(hub.retarget(&second));
    assert!(matches!(hub.state(), State::Idle));
    assert_eq!(hub.target(), &second);
    let other = ConnectTarget::Ssh {
        target: "user@other".into(),
        session: "second".into(),
    };
    assert!(hub.retarget(&other));
}

static DETACH_CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn detached_join(_: &ConnectTarget, _: Start) -> Result<CoreBackend, Error> {
    DETACH_CALLS.fetch_add(1, Ordering::Relaxed);
    Err(Error::Namespace)
}

#[test]
fn detached_tabs_wait_for_an_explicit_connect_request() {
    let mut hub = Hub::with_join(detached_join);
    hub.detach();
    hub.connect(Start::Join);
    assert!(matches!(hub.state(), State::Detached));
    assert_eq!(DETACH_CALLS.load(Ordering::Relaxed), 0);
    hub.connect(Start::IfMissing);
    settle(&mut hub);
    assert_eq!(DETACH_CALLS.load(Ordering::Relaxed), 1);
}
