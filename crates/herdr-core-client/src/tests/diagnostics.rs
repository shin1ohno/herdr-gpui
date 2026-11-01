use super::*;
use crate::{Error, Method};

#[test]
fn remote_error_diagnostics_are_bounded_without_changing_their_kind() {
    let (client, mut peer) = connected();
    let call = std::thread::spawn(move || client.doc_get("bounded"));
    let request = peer.request();
    let reply = json!({"jsonrpc": "2.0", "id": request["id"], "error": {
        "code": -32000, "message": "日本語".repeat(4096), "data": {"kind": "unavailable"}
    }});
    peer.send(0, Kind::Data, &serde_json::to_vec(&reply).unwrap());
    let Err(Error::Refused {
        method,
        kind,
        message,
    }) = call.join().unwrap()
    else {
        panic!("expected a typed remote refusal");
    };
    assert_eq!(method, Method::DocGet);
    assert_eq!(kind.as_deref(), Some("unavailable"));
    assert!(message.len() <= 4096);
    assert!(message.ends_with('…'));
}
