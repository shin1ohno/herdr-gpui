use super::gate_transport::{attached, eof, until};
use super::*;
use crate::{Delivery, Error};
use std::{
    sync::{Arc, atomic::Ordering},
    thread,
};

#[test]
fn cancelled_queued_frame_is_discarded_but_following_input_survives() {
    let (core, attachment, mut peer, control) = attached();
    control.park_read();
    let gate = attachment.write_gated(b"cancel me").unwrap();
    assert_eq!(gate.cancel(), Delivery::Dropped);
    assert!(gate.is_finished());
    attachment.write(b"after").unwrap();
    control.release();
    assert_eq!(peer.frame().payload, b"after");
    assert!(!core.is_lost());
}

#[test]
fn cancellation_during_partial_write_closes_connection_and_never_completes_frame() {
    let (core, attachment, mut peer, control) = attached();
    control.write.store(1, Ordering::Release);
    let gate = attachment.write_gated(b"never complete").unwrap();
    control.wait();
    assert_eq!(gate.cancel(), Delivery::Writing);
    control.release();
    assert_eq!(eof(&mut peer.socket).len(), 1);
    until(|| core.is_lost() && gate.is_finished());
    assert_eq!(gate.state(), Delivery::Interrupted);
}

#[test]
fn cancellation_after_final_write_has_no_effect_even_before_write_returns() {
    let (core, attachment, mut peer, control) = attached();
    control.write.store(2, Ordering::Release);
    let gate = attachment.write_gated(b"complete").unwrap();
    control.wait();
    assert_eq!(gate.cancel(), Delivery::Writing);
    control.release();
    until(|| gate.is_finished());
    assert_eq!(gate.state(), Delivery::Written);
    assert_eq!(gate.cancel(), Delivery::Written);
    attachment.write(b"after").unwrap();
    assert_eq!(peer.frame().payload, b"complete");
    assert_eq!(peer.frame().payload, b"after");
    assert!(!core.is_lost());
}

#[test]
fn disconnect_discards_queued_gate_without_writing() {
    let (core, attachment, mut peer, control) = attached();
    control.park_read();
    let gate = attachment.write_gated(b"queued").unwrap();
    core.disconnect();
    control.release();
    assert!(eof(&mut peer.socket).is_empty());
    until(|| gate.is_finished());
    assert_eq!(gate.state(), Delivery::Dropped);
}

#[test]
fn full_queue_returns_busy_without_a_gate() {
    let (_core, attachment, mut peer, control) = attached();
    control.park_read();
    for _ in 0..1024 {
        attachment.write(b"q").unwrap();
    }
    assert!(matches!(
        attachment.write_gated(b"must not queue"),
        Err(Error::Busy)
    ));
    control.release();
    for _ in 0..1024 {
        assert_eq!(peer.frame().payload, b"q");
    }
    attachment.write(b"after").unwrap();
    assert_eq!(peer.frame().payload, b"after");
}

#[test]
fn short_writes_and_io_interrupted_retry_but_zero_write_ends_connection() {
    for mode in [3, 4, 5] {
        let (core, attachment, mut peer, control) = attached();
        control.write.store(mode, Ordering::Release);
        let gate = attachment.write_gated(b"whole frame").unwrap();
        if mode == 4 {
            assert!(eof(&mut peer.socket).is_empty());
            until(|| gate.is_finished());
            assert_eq!(gate.state(), Delivery::Interrupted);
            assert!(core.is_lost());
        } else {
            assert_eq!(peer.frame().payload, b"whole frame");
            until(|| gate.is_finished());
            assert_eq!(gate.state(), Delivery::Written);
        }
    }
}

#[test]
fn put_in_flight_limit_does_not_send_the_sixty_fifth_request() {
    let (client, mut peer) = connected();
    let client = Arc::new(client);
    let mut calls = Vec::new();
    for _ in 0..64 {
        let core = client.clone();
        calls.push(thread::spawn(move || core.put("t7", "p", b"x")));
        assert_eq!(peer.request()["method"], "terminal.put");
    }
    assert!(matches!(
        client.put("t7", "must not send", b"x"),
        Err(Error::Busy)
    ));
    client.disconnect();
    assert!(eof(&mut peer.socket).is_empty());
    for call in calls {
        assert!(matches!(call.join().unwrap(), Err(Error::Lost)));
    }
}

#[test]
fn disconnect_racing_with_read_response_fences_later_calls() {
    let (core, _attachment, mut peer, control) = attached();
    let core = Arc::new(core);
    let caller = core.clone();
    let call = thread::spawn(move || caller.put("t7", "p", b"x"));
    let request = peer.request();
    control.after_read.store(true, Ordering::Release);
    peer.answer(&request, json!({"path":"/p", "bytes":1}));
    control.wait();
    core.disconnect();
    control.release();
    let result = call.join().unwrap();
    eprintln!("in-flight result after disconnect: {result:?}");
    assert!(matches!(core.put("t7", "never", b"x"), Err(Error::Lost)));
    assert!(eof(&mut peer.socket).is_empty());
}
