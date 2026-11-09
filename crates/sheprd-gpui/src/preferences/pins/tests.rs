#![allow(clippy::unwrap_used)]
use super::*;

#[test]
fn newest_pin_wins_unpin_restores_and_repin_is_newest() {
    let mut pins = AgentPins::default();
    let a = AgentPin::new("local", "boot", "a");
    let b = AgentPin::new("local", "boot", "b");
    pins.toggle(a.clone());
    pins.toggle(b.clone());
    assert_eq!(pins.rank("local", "boot", "b"), Some(0));
    assert_eq!(pins.rank("local", "boot", "a"), Some(1));
    pins.toggle(a.clone());
    assert_eq!(pins.rank("local", "boot", "a"), None);
    pins.toggle(a);
    assert_eq!(pins.rank("local", "boot", "a"), Some(0));
    assert_eq!(pins.rank("local", "boot", "b"), Some(1));
    assert_eq!(pins.rank("ssh:other", "boot", "a"), None);
    assert_eq!(pins.rank("local", "new-boot", "a"), None);
}

#[test]
fn persistence_keeps_order_ignores_malformed_duplicates_and_legacy() {
    let mut pins = AgentPins::default();
    pins.toggle(AgentPin::new("local", "boot", "a"));
    pins.toggle(AgentPin::new("remote", "boot", "a"));
    let value = serde_json::to_value(&pins).unwrap();
    assert_eq!(AgentPins::read(Some(&value)), pins);
    assert_eq!(AgentPins::read(None), AgentPins::default());
    let bad = serde_json::json!([value[0], {}, value[0], value[1], {"endpoint":"", "boot":"b", "pane":"p"}]);
    assert_eq!(AgentPins::read(Some(&bad)), pins);
}

#[test]
fn closed_panes_are_retired_without_pruning_another_host_or_session() {
    let mut pins = AgentPins::default();
    for (host, boot) in [
        ("local", "old"),
        ("remote", "old"),
        ("local", "other-session"),
    ] {
        pins.toggle(AgentPin::new(host, boot, "p0"));
    }
    let mut snapshot = crate::sidebar::layout_tests::snapshot(1);
    snapshot.boot_id = "old".into();
    snapshot.panes.clear();
    assert!(pins.reconcile("local", &snapshot));
    assert_eq!(pins.rank("local", "old", "p0"), None);
    assert!(pins.rank("remote", "old", "p0").is_some());
    assert!(pins.rank("local", "other-session", "p0").is_some());
}
