//! Writers register themselves, and a sync covers every live one.
use super::*;
use std::time::Duration;

fn writer(registry: &sync::Registry, path: PathBuf) -> Writer<Vec<i32>> {
    Writer::spawn("state-registry", path, registry, |path, value| {
        write(path, value)
    })
    .unwrap()
}

#[test]
fn a_sync_covers_every_registered_writer() {
    let dir = tempfile::tempdir().unwrap();
    let registry = sync::Registry::new();
    let paths: Vec<_> = (0..3)
        .map(|index| dir.path().join(format!("{index}.json")))
        .collect();
    let writers: Vec<_> = paths
        .iter()
        .map(|path| writer(&registry, path.clone()))
        .collect();
    for (index, writer) in (0..).zip(&writers) {
        writer.save(vec![index]);
        writer.save(vec![index, index]);
    }
    let report = registry.sync(Duration::from_secs(10));
    assert_eq!(
        report,
        SyncReport {
            lost: 0,
            synced: 3,
            timed_out: 0,
        }
    );
    for (index, path) in (0..).zip(&paths) {
        assert_eq!(saved(path), [index, index]);
    }
}

#[test]
fn dropped_writers_are_pruned() {
    let dir = tempfile::tempdir().unwrap();
    let registry = sync::Registry::new();
    let kept = writer(&registry, dir.path().join("kept.json"));
    let gone = writer(&registry, dir.path().join("gone.json"));
    assert_eq!(registry.len(), 2);
    gone.finish();
    assert_eq!(registry.len(), 1);
    assert_eq!(registry.sync(Duration::from_secs(10)).synced, 1);
    kept.finish();
    assert_eq!(registry.len(), 0);
}

#[test]
fn every_started_writer_joins_the_process_registry() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("global.json");
    let writer = Writer::start("state-global", path.clone()).unwrap();
    // Checked by identity: other tests' writers share the process registry,
    // so a count would pass without this one.
    assert!(sync::REGISTRY.contains(&writer.barrier));
    writer.save(vec![4, 2]);
    let report = sync_all(Duration::from_secs(10));
    assert_eq!(report.timed_out, 0);
    assert_eq!(saved(&path), [4, 2]);
    writer.finish();
}
