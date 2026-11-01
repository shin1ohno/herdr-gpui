//! `Writer::sync_until` waits for the latest saved value, and no longer.
use super::*;
use crate::state_file::sync::Synced;
use std::{
    sync::mpsc::{Receiver, channel},
    time::{Duration, Instant},
};

/// Long enough never to be reached by a sync that should succeed.
const GENEROUS: Duration = Duration::from_secs(10);

fn generous() -> Instant {
    Instant::now() + GENEROUS
}

/// A writer whose writes wait for the test: each write sends its value on
/// the first channel, then blocks until the test sends on the returned one.
fn gated(registry: &sync::Registry) -> (Writer<i32>, Receiver<i32>, mpsc::Sender<()>) {
    let (started, writes) = channel();
    let (release, gate) = channel::<()>();
    let gate = Mutex::new(gate);
    let writer = Writer::spawn(
        "state-gated",
        PathBuf::from("unused.json"),
        registry,
        move |_, value: &i32| {
            started.send(*value).unwrap();
            // A dropped sender also releases the write, so a test that
            // panics does not leave the worker blocked.
            let _ = gate.lock().unwrap().recv();
            Ok(())
        },
    )
    .unwrap();
    (writer, writes, release)
}

#[test]
fn sync_waits_until_the_latest_value_is_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.json");
    let writer = Writer::start("state-sync", path.clone()).unwrap();
    for value in 0..50 {
        writer.save(vec![value]);
    }
    assert_eq!(writer.sync_until(generous()), Synced::Written);
    assert_eq!(saved(&path), [49]);
    writer.save(vec![50, 51]);
    assert_eq!(writer.sync_until(generous()), Synced::Written);
    assert_eq!(saved(&path), [50, 51]);
    writer.finish();
}

#[test]
fn sync_returns_at_once_when_nothing_was_saved() {
    let registry = sync::Registry::new();
    let (writer, _writes, _release) = gated(&registry);
    // A deadline already passed: an idle writer has nothing to wait for.
    assert_eq!(writer.sync_until(Instant::now()), Synced::Written);
}

#[test]
fn sync_returns_false_at_the_deadline_while_a_write_is_stalled() {
    let registry = sync::Registry::new();
    let (writer, writes, release) = gated(&registry);
    writer.save(1);
    assert_eq!(writes.recv_timeout(GENEROUS).unwrap(), 1);

    let started = Instant::now();
    assert_eq!(
        writer.sync_until(started + Duration::from_millis(50)),
        Synced::TimedOut
    );
    assert!(started.elapsed() >= Duration::from_millis(50));
    assert!(started.elapsed() < GENEROUS);

    release.send(()).unwrap();
    assert_eq!(writer.sync_until(generous()), Synced::Written);
}

#[test]
fn sync_waits_for_a_value_saved_during_a_write() {
    let registry = sync::Registry::new();
    let (writer, writes, release) = gated(&registry);
    writer.save(1);
    assert_eq!(writes.recv_timeout(GENEROUS).unwrap(), 1);
    // The worker holds 1; 2 waits in the mailbox behind it.
    writer.save(2);
    release.send(()).unwrap();
    assert_eq!(
        writer.sync_until(Instant::now() + Duration::from_millis(20)),
        Synced::TimedOut
    );
    assert_eq!(writes.recv_timeout(GENEROUS).unwrap(), 2);
    release.send(()).unwrap();
    assert_eq!(writer.sync_until(generous()), Synced::Written);
}

#[test]
fn a_failed_write_still_completes_the_sync() {
    let registry = sync::Registry::new();
    let writer = Writer::spawn(
        "state-failing",
        PathBuf::from("unused.json"),
        &registry,
        |_, _: &i32| Err(crate::Error::MissingStateRoot),
    )
    .unwrap();
    writer.save(1);
    assert_eq!(writer.sync_until(generous()), Synced::Written);
}

#[test]
fn a_finished_writer_leaves_nothing_to_wait_for() {
    let registry = sync::Registry::new();
    let (writer, _writes, release) = gated(&registry);
    drop(release);
    writer.save(1);
    writer.finish();
    assert_eq!(registry.len(), 0);
    assert_eq!(registry.sync(Duration::ZERO), SyncReport::default());
}

#[test]
fn a_writer_whose_worker_ended_without_writing_is_lost() {
    let registry = sync::Registry::new();
    let writer = Writer::spawn(
        "state-panicking",
        PathBuf::from("unused.json"),
        &registry,
        |_, _: &i32| -> crate::Result<()> { panic!("the worker ends") },
    )
    .unwrap();
    writer.save(1);
    // The value is never recorded as written, but the worker is gone, so
    // nothing will ever write it: the sync does not wait for the deadline,
    // and does not count the value as saved either.
    assert_eq!(writer.sync_until(generous()), Synced::Lost);
    assert_eq!(
        registry.sync(GENEROUS),
        SyncReport {
            lost: 1,
            synced: 0,
            timed_out: 0,
        }
    );
}

#[test]
fn a_timeout_too_large_for_a_deadline_still_syncs() {
    let registry = sync::Registry::new();
    let (writer, writes, release) = gated(&registry);
    writer.save(1);
    assert_eq!(writes.recv_timeout(GENEROUS).unwrap(), 1);
    release.send(()).unwrap();
    // `Instant::now() + Duration::MAX` would overflow; the sync waits for the
    // write instead of panicking.
    assert_eq!(registry.sync(Duration::MAX).synced, 1);
}

#[test]
fn a_dropped_writer_is_synced_once_its_worker_ends() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.json");
    let registry = sync::Registry::new();
    let writer = Writer::spawn("state-dropped", path.clone(), &registry, |path, value| {
        write(path, value)
    })
    .unwrap();
    writer.save(vec![7]);
    // Dropping the writer closes its wake channel; the worker still writes
    // the pending value, and a sync meanwhile waits for it.
    drop(writer);
    assert_eq!(registry.sync(GENEROUS).timed_out, 0);
    let deadline = generous();
    while registry.len() > 0 && Instant::now() < deadline {
        thread::yield_now();
    }
    assert_eq!(registry.len(), 0);
    assert_eq!(saved(&path), [7]);
}
