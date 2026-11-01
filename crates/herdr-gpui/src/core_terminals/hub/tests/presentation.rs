use super::*;
use crate::{sidebar::layout_tests::fixture_window, state::ConnectionStatus};
use herdr_core_client::{Core, Method};
use serde_json::json;
use std::{io::Read as _, io::Write as _, os::unix::net::UnixStream};

fn ready_backend() -> (CoreBackend, UnixStream) {
    let (client, mut server) = UnixStream::pair().unwrap();
    client
        .set_read_timeout(Some(Duration::from_millis(20)))
        .unwrap();
    server
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let peer = std::thread::spawn(move || {
        let mut line = Vec::new();
        while !line.ends_with(b"\n") {
            let mut byte = [0];
            server.read_exact(&mut byte).unwrap();
            line.push(byte[0]);
            assert!(line.len() < 64 << 10);
        }
        let request: serde_json::Value = serde_json::from_slice(&line).unwrap();
        let reply = json!({"jsonrpc": "2.0", "id": request["id"], "result": {
            "generation": 1, "version": "0.1.13", "framed": true, "features": [],
            "core": {"id": "c0ffee", "name": "test"},
            "methods": Method::ALL.iter().map(|method| method.as_str()).collect::<Vec<_>>()}});
        server
            .write_all(&serde_json::to_vec(&reply).unwrap())
            .unwrap();
        server.write_all(b"\n").unwrap();
        server
    });
    let core = Core::over(client).unwrap();
    (CoreBackend::new(core, SCROLLBACK), peer.join().unwrap())
}

#[gpui::test]
fn core_connection_presentation_follows_the_live_core_in_its_exact_namespace(
    cx: &mut gpui::TestAppContext,
) {
    let (backend, _peer) = ready_backend();
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, _| {
        let target = ConnectTarget::Ssh {
            target: "user@box".into(),
            session: "work".into(),
        };
        view.endpoints[0].id = "ssh:core-presentation".into();
        view.endpoints[0].connection.target = target.clone();
        view.core_terminals.hub.retarget(&target);
        view.core_terminals.scope = Some(crate::browser::scope(&view.endpoints[0]));
        view.core_terminals.workspace = Some("herdr-ui:core:work".into());
        view.live.status = ConnectionStatus::Connecting;
        view.core_terminals.hub.state = State::Ready(backend);
        assert_eq!(
            view.core_connection_status("ssh:core-presentation"),
            ConnectionStatus::Connected
        );
        assert_eq!(
            view.displayed_connection_status(&view.endpoints[0]),
            if cfg!(target_os = "ios") {
                ConnectionStatus::Connected
            } else {
                ConnectionStatus::Connecting
            }
        );
        assert_eq!(
            view.core_connection_status("ssh:other"),
            ConnectionStatus::Disconnected
        );
        view.core_terminals.hub.detach();
        assert_eq!(
            view.core_connection_status("ssh:core-presentation"),
            ConnectionStatus::Detached
        );
        view.core_terminals.hub.reset();
        assert_eq!(
            view.core_connection_status("ssh:core-presentation"),
            ConnectionStatus::Disconnected
        );
        assert_eq!(view.live.status, ConnectionStatus::Connecting);
    });
}
