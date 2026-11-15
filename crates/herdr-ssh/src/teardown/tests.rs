#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{collections::HashSet, thread, time::Instant};

use super::*;

#[test]
fn identities_never_repeat_across_threads() {
    let counter = Arc::new(AtomicU64::new(1));
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let counter = Arc::clone(&counter);
            thread::spawn(move || {
                (0..1000)
                    .map(|_| allocate(&counter).unwrap())
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    let mut seen = HashSet::new();
    for worker in workers {
        for id in worker.join().unwrap() {
            assert!(seen.insert(id), "{id:?} was handed out twice");
        }
    }
    assert_eq!(seen.len(), 8000);
}

#[test]
fn identities_run_out_instead_of_wrapping() {
    let counter = AtomicU64::new(u64::MAX - 1);
    assert_eq!(allocate(&counter).unwrap(), TunnelId(u64::MAX - 1));
    assert_eq!(allocate(&counter), Err(Error::IdsExhausted));
    assert_eq!(allocate(&counter), Err(Error::IdsExhausted));
    assert_eq!(counter.load(Ordering::Relaxed), u64::MAX);
}

#[test]
fn the_first_outcome_stays_and_every_wait_sees_it() {
    let slot = TeardownSlot::default();
    let tunnel = TunnelId(7);
    let started = Instant::now();
    assert!(slot.wait(tunnel, Duration::from_millis(50)).is_none());
    assert!(started.elapsed() >= Duration::from_millis(50));
    slot.settle(Outcome::Complete);
    slot.settle(Outcome::Unproven);
    for _ in 0..3 {
        let Some(Teardown::Complete(receipt)) = slot.wait(tunnel, Duration::ZERO) else {
            panic!("the complete outcome was replaced or lost");
        };
        assert_eq!(receipt.tunnel(), tunnel);
    }
}

#[test]
fn a_dropped_fuse_settles_unproven_but_never_replaces_an_outcome() {
    let slot = Arc::new(TeardownSlot::default());
    drop(Fuse::new(Arc::clone(&slot), Outcome::Unproven));
    assert!(matches!(
        slot.wait(TunnelId(1), Duration::ZERO),
        Some(Teardown::Unproven)
    ));
    let slot = Arc::new(TeardownSlot::default());
    Fuse::new(Arc::clone(&slot), Outcome::Unproven).settle(Outcome::Complete);
    assert!(matches!(
        slot.wait(TunnelId(1), Duration::ZERO),
        Some(Teardown::Complete(_))
    ));
}

#[test]
fn sign_tracker_is_idle_only_when_every_guard_dropped() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    let tracker = SignTracker::default();
    let first = tracker.start();
    let second = tracker.start();
    runtime.block_on(async {
        let short = Duration::from_millis(20);
        assert!(tokio::time::timeout(short, tracker.idle()).await.is_err());
        drop(first);
        assert!(tokio::time::timeout(short, tracker.idle()).await.is_err());
        drop(second);
        assert!(tokio::time::timeout(short, tracker.idle()).await.is_ok());
    });
}

#[test]
fn a_stop_before_waiting_is_seen() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap();
    let stop = StopSignal::default();
    stop.stop();
    runtime.block_on(async {
        assert!(
            tokio::time::timeout(Duration::from_millis(20), stop.stopped())
                .await
                .is_ok()
        );
    });
}
