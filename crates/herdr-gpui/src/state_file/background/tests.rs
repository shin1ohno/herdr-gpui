#![allow(clippy::unwrap_used)]
use super::*;
use crate::state_file::{Writer, read};
use gpui::TestAppContext;

fn saved(path: &std::path::Path) -> Vec<String> {
    serde_json::from_slice(&read(path, 1024).unwrap().unwrap()).unwrap()
}

#[gpui::test]
fn flush_writes_every_store_and_leaves_them_running(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    let tabs_path = dir.path().join("tabs.json");
    let frecency_path = dir.path().join("frecency.json");
    let tabs = Writer::start("flush-tabs", tabs_path.clone()).unwrap();
    let frecency = Writer::start("flush-frecency", frecency_path.clone()).unwrap();
    // By identity: other tests' writers share the process registry, so the
    // report's counts would pass without these two.
    assert!(crate::state_file::sync::REGISTRY.contains(&tabs.barrier));
    assert!(crate::state_file::sync::REGISTRY.contains(&frecency.barrier));
    tabs.save(vec!["first".to_owned()]);
    tabs.save(vec!["first".to_owned(), "second".to_owned()]);
    frecency.save(vec!["palette".to_owned()]);

    let flush = cx.update(|cx| flush(cx, Duration::from_secs(10)));
    let (done, result) = std::sync::mpsc::sync_channel(1);
    cx.executor()
        .spawn(async move { done.send(flush.await).unwrap() })
        .detach();
    cx.run_until_parked();
    let flushed = result.try_recv().unwrap();

    // No Settings window is open, so there are no drafts to wait for.
    assert!(flushed.drafts);
    assert_eq!(flushed.stores.timed_out, 0, "{flushed:?}");
    assert_eq!(flushed.stores.lost, 0, "{flushed:?}");
    assert_eq!(saved(&tabs_path), ["first", "second"]);
    assert_eq!(saved(&frecency_path), ["palette"]);

    // Unlike the quit hooks, a flush does not stop the writers.
    tabs.save(vec!["after".to_owned()]);
    tabs.finish();
    frecency.finish();
    assert_eq!(saved(&tabs_path), ["after"]);
}
