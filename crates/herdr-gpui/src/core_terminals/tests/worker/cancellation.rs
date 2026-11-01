use super::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc,
};

#[test]
fn dropping_a_link_during_input_does_not_flush_the_remaining_commands() {
    let (entered, first_input) = mpsc::sync_channel(1);
    let (resume, release) = mpsc::sync_channel(1);
    let release = Mutex::new(release);
    let held = AtomicBool::new(false);
    let backend = Scripted {
        on_input: Some(Arc::new(move || {
            if !held.swap(true, Ordering::AcqRel) {
                entered.send(()).unwrap();
                release
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(5))
                    .unwrap();
            }
        })),
        ..Default::default()
    };
    let link = worker::spawn(
        backend.clone(),
        TerminalId::new("t1").unwrap(),
        size(80, 24),
        Role::Active,
    )
    .unwrap();
    let first = Command::Input(ClientPaneInputEvent::TextCommit("first".into()));
    link.send(first.clone());
    first_input.recv_timeout(Duration::from_secs(5)).unwrap();
    link.send(Command::Input(ClientPaneInputEvent::TextCommit(
        "must not type".into(),
    )));
    link.send(Command::Resize(size(100, 30)));
    link.send(Command::Focus(true));
    drop(link);
    resume.send(()).unwrap();
    until(|| (Arc::strong_count(&backend.received) == 1).then_some(()));
    assert_eq!(
        *backend.received.lock().unwrap(),
        [Command::Resize(size(80, 24)), first]
    );
}
