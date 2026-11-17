use super::{MockCore, connected, terminal};
use crate::{
    CoreBackend, CoreStatus,
    backend::info,
    frame::Kind,
    wire::{Status, StatusState, Terminal},
};
use serde_json::{Value, json};
use sheprd_backend::{AgentStatus, Backend, Changes};
use std::{thread, time::Duration};

fn backend() -> (CoreBackend, MockCore) {
    let (core, mock) = connected();
    (CoreBackend::new(core, 1000), mock)
}

fn with_status(address: &str, status: Option<Value>) -> Value {
    let mut record = terminal(address);
    if let Some(status) = status {
        record["status"] = status;
    }
    record
}

/// `statuses()` against a core that lists `terminals`.
fn listed(terminals: &[Value]) -> Vec<CoreStatus> {
    let (backend, mut mock) = backend();
    thread::scope(|scope| {
        let call = scope.spawn(|| backend.statuses());
        let request = mock.request();
        assert_eq!(request["method"], "terminal.list");
        mock.answer(&request, json!({"terminals": terminals}));
        call.join().unwrap().unwrap()
    })
}

/// Watches `terminals` at seq 10 on channel 3, then reads one event per
/// `(type, address, data)` and returns the changes with them applied.
fn after_events(terminals: &[Value], events: &[(&str, &str, Value)]) -> crate::CoreChanges {
    let (backend, mut mock) = backend();
    let mut changes = thread::scope(|scope| {
        let watch = scope.spawn(|| backend.watch());
        let request = mock.request();
        assert_eq!(request["method"], "events.subscribe");
        mock.answer(
            &request,
            json!({"channel": 3, "seq": 10, "terminals": terminals}),
        );
        watch.join().unwrap().unwrap().1
    });
    let bytes = events
        .iter()
        .zip(11..)
        .flat_map(|((kind, address, data), seq)| {
            let message = json!({"method": "event", "params": {
                "seq": seq, "type": kind, "terminal": address, "data": data}});
            [serde_json::to_vec(&message).unwrap(), b"\n".to_vec()].concat()
        })
        .collect::<Vec<u8>>();
    mock.send(3, Kind::Data, &bytes);
    let mut read = 0;
    for _ in 0..100 {
        read += changes.next(Duration::from_millis(50)).unwrap().len();
        if read >= events.len() {
            break;
        }
    }
    assert_eq!(read, events.len(), "every event changes a record");
    changes
}

#[test]
fn raw_status_keeps_error() {
    let listed = listed(&[with_status("t1", Some(json!({"state": "error"})))]);
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].address, "t1");
    assert_eq!(
        listed[0].status.as_ref().map(|status| status.state),
        Some(StatusState::Error)
    );

    // The same terminal after a status event stays an error too.
    let changes = after_events(
        &[with_status("t1", Some(json!({"state": "working"})))],
        &[(
            "status",
            "t1",
            json!({"status": {"state": "error"}, "reported": true}),
        )],
    );
    let current = changes.status("t1").unwrap();
    assert_eq!(
        current.status.as_ref().map(|status| status.state),
        Some(StatusState::Error)
    );
    assert_eq!(changes.statuses(), vec![current]);
}

#[test]
fn raw_status_none_when_unreported() {
    let listed = listed(&[
        with_status("t1", None),
        with_status("t2", Some(json!({"state": "idle"}))),
    ]);
    assert_eq!(listed[0].status, None);
    assert_eq!(
        listed[1].status.as_ref().map(|status| status.state),
        Some(StatusState::Idle)
    );

    // A cleared status and a removed terminal read as no status and none.
    let changes = after_events(
        &[
            with_status("t1", Some(json!({"state": "working"}))),
            with_status("t2", Some(json!({"state": "working"}))),
        ],
        &[
            ("status", "t1", json!({"status": null, "reported": false})),
            ("removed", "t2", json!({})),
        ],
    );
    assert_eq!(changes.status("t1").unwrap().status, None);
    assert_eq!(changes.status("t2"), None);
    assert_eq!(changes.statuses().len(), 1);
}

#[test]
fn raw_status_keeps_app_kind_msg() {
    let reported = json!({
        "state": "blocked", "app": "claude", "kind": "permission",
        "msg": "Run rm?", "progress": 40, "title": "cleanup"});
    let expected = Status {
        app: "claude".to_owned(),
        kind: Some("permission".to_owned()),
        msg: "Run rm?".to_owned(),
        progress: Some(40),
        state: StatusState::Blocked,
        title: "cleanup".to_owned(),
    };
    let listed = listed(&[with_status("t1", Some(reported.clone()))]);
    assert_eq!(listed[0].status.as_ref(), Some(&expected));

    let changes = after_events(
        &[with_status("t1", None)],
        &[(
            "status",
            "t1",
            json!({"status": reported, "reported": true}),
        )],
    );
    assert_eq!(changes.status("t1").unwrap().status, Some(expected));
}

/// The shared mapping a backend-agnostic consumer reads stays as it was:
/// error folds into blocked, and no status is unknown.
#[test]
fn terminal_info_agent_mapping_unchanged() {
    let cases = [
        (None, AgentStatus::Unknown),
        (Some("blocked"), AgentStatus::Blocked),
        (Some("error"), AgentStatus::Blocked),
        (Some("done"), AgentStatus::Done),
        (Some("idle"), AgentStatus::Idle),
        (Some("working"), AgentStatus::Working),
        (Some("something_new"), AgentStatus::Unknown),
    ];
    for (state, agent) in cases {
        let record: Terminal = serde_json::from_value(with_status(
            "t1",
            state.map(|state| json!({"state": state})),
        ))
        .unwrap();
        assert_eq!(info(&record).unwrap().agent, agent, "{state:?}");
    }
}
