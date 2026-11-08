use super::super::supports_core_target;
#[cfg(not(target_os = "ios"))]
use super::super::{Error, Hub, Start, State, fallback, panes::Panes};
#[cfg(not(target_os = "ios"))]
use crate::browser::{Location, Store};
#[cfg(not(target_os = "ios"))]
use sheprd_backend::TerminalId;
use sheprd_client::ConnectTarget;
#[cfg(not(target_os = "ios"))]
use std::sync::Arc;

#[test]
fn core_transport_support_follows_the_endpoint_machine() {
    let ssh = ConnectTarget::Ssh {
        target: "user@remote".into(),
        session: "default".into(),
    };
    assert_eq!(supports_core_target(&ssh), cfg!(target_os = "ios"));
    for target in [
        ConnectTarget::Local,
        ConnectTarget::Socket("/unused-target-boundary.sock".into()),
        ConnectTarget::Session {
            name: "work".into(),
            development: false,
        },
    ] {
        assert_eq!(supports_core_target(&target), !cfg!(target_os = "ios"));
    }
    assert!(!supports_core_target(&ConnectTarget::Wsl {
        distro: "Ubuntu".into(),
        session: "default".into(),
    }));
}

#[cfg(not(target_os = "ios"))]
fn fake_join(_: &ConnectTarget, _: Start) -> Result<sheprd_core_client::CoreBackend, Error> {
    // A broken boundary remains isolated from personal core discovery.
    Err(Error::UnsupportedTarget)
}

#[cfg(not(target_os = "ios"))]
fn remote_workspace(view: &mut crate::SheprdWindow, target: ConnectTarget) {
    let mut snapshot = crate::sidebar::layout_tests::ssh_snapshot(1);
    snapshot.focused_workspace_id = Some(snapshot.workspaces[0].workspace_id.clone());
    snapshot.focused_tab_id = Some(snapshot.tabs[0].tab_id.clone());
    let mut live = crate::state::LiveState::default();
    live.snapshot = Some(Arc::new(snapshot));
    live.status = crate::state::ConnectionStatus::Connected;
    live.supports_surface = true;
    let endpoint = &mut view.endpoints[0];
    endpoint.id = "ssh:target-boundary".into();
    endpoint.connection.target = target;
    endpoint.live = live.clone();
    *endpoint.connection.inbox.lock().unwrap() = live.clone();
    view.live = live;
}

#[cfg(not(target_os = "ios"))]
#[gpui::test]
fn desktop_remote_creation_and_reconnect_never_connect_to_the_local_core(
    cx: &mut gpui::TestAppContext,
) {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    view.update_in(cx, |view, window, cx| {
        view.core_terminals.hub = Hub::with_join(fake_join);
        for target in [
            ConnectTarget::Ssh {
                target: "user@remote".into(),
                session: "default".into(),
            },
            ConnectTarget::Wsl {
                distro: "Ubuntu".into(),
                session: "default".into(),
            },
        ] {
            remote_workspace(view, target);
            assert!(view.browser_key().is_some(), "the remote workspace exists");
            view.new_core_terminal(window, cx);
            assert!(matches!(view.core_terminals.hub.state(), State::Idle));
            assert!(view.core_terminals.wanted.is_none());
            assert!(view.core_terminals.starting.is_none());
            view.reconnect_core_connection(cx);
            assert!(matches!(view.core_terminals.hub.state(), State::Idle));
        }
    });
}

#[cfg(not(target_os = "ios"))]
#[gpui::test]
fn desktop_remote_restored_tabs_and_missing_installation_never_use_the_local_core(
    cx: &mut gpui::TestAppContext,
) {
    let (view, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    view.update(cx, |view, cx| {
        Store::default().install(cx);
        view.core_terminals.hub = Hub::with_join(fake_join);
        remote_workspace(
            view,
            ConnectTarget::Ssh {
                target: "user@remote".into(),
                session: "default".into(),
            },
        );
        let (scope, workspace) = view.browser_key().unwrap();
        let tab = Store::update(cx, |store| {
            store.open(
                scope,
                &workspace,
                Some(Location::CoreTerminal {
                    panes: Panes::single(TerminalId::new("t1").unwrap()),
                    observe: false,
                    remote: None,
                }),
                None,
            )
        })
        .unwrap();
        view.sync_core_views(cx);
        assert!(!view.core_terminals.tabs.contains_key(&tab));
        assert!(view.core_terminals.views.is_empty());
        assert!(matches!(view.core_terminals.hub.state(), State::Idle));
        view.live.missing_installation = true;
        view.poll_core_fallback(cx);
        assert!(matches!(view.core_terminals.hub.state(), State::Idle));
        assert!(matches!(
            view.core_terminals.fallback,
            fallback::Fallback::Off
        ));
    });
}
