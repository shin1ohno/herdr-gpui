#![allow(clippy::unwrap_used)]
use super::*;

mod barrier;
mod registry;

/// The decoded contents of a state file written by a test writer.
fn saved(path: &Path) -> Vec<i32> {
    let bytes = read(path, 1024).unwrap().unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[test]
fn missing_files_are_empty_and_oversized_files_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.json");
    assert!(read(&path, 8).unwrap().is_none());
    fs::write(&path, b"123456789").unwrap();
    assert!(matches!(
        read(&path, 8),
        Err(crate::Error::StateFileSize { limit: 8, .. })
    ));
    assert_eq!(read(&path, 9).unwrap().unwrap(), b"123456789");
}

#[test]
fn the_latest_value_is_written_before_finish_returns() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested/state.json");
    let writer = Writer::start("state-test", path.clone()).unwrap();
    writer.save(vec![1]);
    writer.save(vec![2, 3]);
    writer.finish();
    assert_eq!(saved(&path), [2, 3]);
}
