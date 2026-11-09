use super::*;
use crate::browser::Location;
use serde_json::json;

fn remote(host: &str, session: &str) -> Remote {
    Remote::from_target(&ConnectTarget::Ssh {
        target: host.into(),
        session: session.into(),
    })
    .unwrap()
}

#[test]
fn saved_remote_tabs_keep_the_host_and_session_namespace() {
    let tab: Location = serde_json::from_value(json!({
        "kind": "core_terminal", "panes": "t1", "remote": {"target": "user@box", "session": "first"}
    }))
    .unwrap();
    let Location::CoreTerminal {
        remote: Some(origin),
        ..
    } = &tab
    else {
        panic!("missing remote origin")
    };
    assert_eq!(*origin, remote("user@box", "first"));
    assert_ne!(*origin, remote("user@box", "second"));
    assert_ne!(*origin, remote("user@other", "first"));
    assert_eq!(
        serde_json::from_value::<Location>(serde_json::to_value(&tab).unwrap()).unwrap(),
        tab
    );
    let legacy: Location =
        serde_json::from_value(json!({"kind": "core_terminal", "terminal": "t1"})).unwrap();
    assert!(matches!(
        legacy,
        Location::CoreTerminal { remote: None, .. }
    ));
}

#[test]
fn core_bridge_uses_the_existing_cli_and_validates_the_session() {
    let command = remote("user@box", "work").command().unwrap();
    assert!(command.starts_with("/bin/sh -c "));
    assert!(command.contains("connect --session"));
    assert!(!command.contains("server start"));
    for session in ["", "../other", "x';touch /tmp/unwanted", "a\nb"] {
        assert!(remote("user@box", session).command().is_err());
    }
}

#[test]
fn legacy_tabs_never_attach_to_a_remote_core() {
    let target = ConnectTarget::Ssh {
        target: "user@box".into(),
        session: "first".into(),
    };
    assert!(!Remote::matches_target(None, &target));
    assert!(Remote::matches_target(None, &ConnectTarget::Local));
    assert!(Remote::matches_target(
        Some(&remote("user@box", "first")),
        &target
    ));
    assert!(!Remote::matches_target(
        Some(&remote("user@box", "second")),
        &target
    ));
    assert!(!Remote::matches_target(
        Some(&remote("user@other", "first")),
        &target
    ));
    assert!(!Remote::matches_target(
        Some(&remote("user@box", "first")),
        &ConnectTarget::Local
    ));
}
