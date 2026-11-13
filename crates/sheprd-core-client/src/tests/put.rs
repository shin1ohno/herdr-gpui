use super::*;
use crate::{Error, Method};
use std::thread;

#[test]
fn put_sends_standard_padded_base64_and_parses_the_receipt() {
    let (client, mut peer) = connected();
    let call = thread::spawn(move || client.put("box/t7", "paste.png", &[0xfb, 0xff]));
    let request = peer.request();
    assert_eq!(request["method"], "terminal.put");
    assert_eq!(
        request["params"],
        json!({"terminal":"box/t7", "name":"paste.png", "data":"+/8="})
    );
    peer.answer(
        &request,
        json!({"path":"/state with space/paste.png", "bytes":2}),
    );
    let put = call.join().unwrap().unwrap();
    assert_eq!(put.path, "/state with space/paste.png");
    assert_eq!(put.bytes, 2);
}

#[test]
fn put_rejects_malformed_receipts() {
    for receipt in [
        json!({"path":"/p"}),
        json!({"path":"/p", "bytes":"2"}),
        json!({"bytes":2}),
    ] {
        let (client, mut peer) = connected();
        let call = thread::spawn(move || client.put("t7", "paste.png", b"hi"));
        let request = peer.request();
        peer.answer(&request, receipt);
        assert!(matches!(
            call.join().unwrap(),
            Err(Error::Invalid(Method::TerminalPut, _))
        ));
    }
}

#[test]
fn put_preserves_each_host_refusal() {
    for kind in [
        "finished",
        "not_found",
        "machine_unknown",
        "invalid_params",
        "internal",
    ] {
        let (client, mut peer) = connected();
        let call = thread::spawn(move || client.put("t7", "paste.png", b"hi"));
        let request = peer.request();
        peer.send(
            0,
            Kind::Data,
            &serde_json::to_vec(&json!({"jsonrpc":"2.0", "id":request["id"],
            "error":{"code":-32000,"message":"refused", "data":{"kind":kind}}}))
            .unwrap(),
        );
        match call.join().unwrap() {
            Err(Error::Refused {
                method: Method::TerminalPut,
                kind: actual,
                message,
            }) => {
                assert_eq!(actual.as_deref(), Some(kind));
                assert_eq!(message, "refused");
            }
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn put_is_optional_in_the_hello() {
    let methods: Vec<_> = every_method()
        .into_iter()
        .filter(|name| *name != "terminal.put")
        .collect();
    let hello: crate::hello::Hello =
        serde_json::from_value(hello_reply("0.1.13", &methods)["result"].clone()).unwrap();
    assert_eq!(hello.check(), Ok(()));
}
