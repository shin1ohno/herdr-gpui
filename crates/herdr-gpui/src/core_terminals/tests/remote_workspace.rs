use crate::{browser::Scope, sidebar::layout_tests::fixture_window};
use herdr_client::ConnectTarget;

#[gpui::test]
fn a_core_only_workspace_is_explicit_client_state_fenced_to_the_host_session(
    cx: &mut gpui::TestAppContext,
) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, _| {
        let target = ConnectTarget::Ssh {
            target: "user@box".into(),
            session: "first".into(),
        };
        view.endpoints[0].id = "ssh:core-only".into();
        view.endpoints[0].connection.target = target.clone();
        view.live.snapshot = None;
        view.live.missing_installation = true;
        assert_eq!(
            view.browser_key(),
            None,
            "a missing Herdr 1 never selects Herdr 2"
        );
        view.core_terminals.hub.retarget(&target);
        view.core_terminals.scope = Some(Scope::endpoint("ssh:core-only"));
        assert!(view.open_core_workspace().is_some());
        assert!(matches!(
            view.core_terminals.hub.state(),
            crate::core_terminals::hub::State::Idle
        ));
        assert!(view.core_terminals.wanted.is_none());
        assert!(view.core_terminals.starting.is_none());
        assert_eq!(
            view.browser_key(),
            Some((
                Scope::endpoint("ssh:core-only"),
                "herdr-ui:core:first".into()
            ))
        );
        assert_eq!(
            view.core_connection_status("ssh:other-profile"),
            crate::state::ConnectionStatus::Disconnected
        );
        assert_eq!(
            view.core_connection_status("ssh:core-only"),
            crate::state::ConnectionStatus::Disconnected
        );
        assert!(
            view.live.snapshot.is_none(),
            "no daemon workspace was fabricated"
        );
        view.endpoints[0].connection.target = ConnectTarget::Ssh {
            target: "user@box".into(),
            session: "second".into(),
        };
        assert_eq!(
            view.core_workspace_key(),
            None,
            "another session cannot use this namespace"
        );
        view.endpoints[0].connection.target = target;
        view.endpoints[0].id = "ssh:other-profile".into();
        assert_eq!(
            view.core_workspace_key(),
            None,
            "endpoint scopes are fenced too"
        );
        view.endpoints[0].id = "ssh:core-only".into();
        view.leave_core_workspace();
        assert_eq!(view.browser_key(), None);
    });
}

#[gpui::test]
fn a_late_terminal_start_cannot_land_in_another_endpoint(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, cx| {
        view.select_core_target(cx);
        let scope = view.core_terminals.scope.clone().unwrap();
        let (late, pending) = std::sync::mpsc::sync_channel(1);
        view.core_terminals.starting = Some((
            crate::core_terminals::Purpose::Tab {
                scope,
                workspace: "old".into(),
            },
            pending,
        ));
        view.endpoints[0].id = "ssh:other-profile".into();
        view.endpoints[0].connection.target = ConnectTarget::Ssh {
            target: "user@other".into(),
            session: "second".into(),
        };
        assert!(view.select_core_target(cx));
        assert!(view.core_terminals.starting.is_none());
        assert!(view.core_terminals.wanted.is_none());
        assert!(
            late.send(Err(herdr_core_client::Error::Lost)).is_err(),
            "the old result has no receiver in the new endpoint"
        );
    });
}

#[gpui::test]
fn a_backend_change_retires_pending_terminal_work_without_starting_another(
    cx: &mut gpui::TestAppContext,
) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, cx| {
        view.select_core_target(cx);
        let scope = view.core_terminals.scope.clone().unwrap();
        let (late, pending) = std::sync::mpsc::sync_channel(1);
        view.core_terminals.workspace = Some("old".into());
        view.core_terminals.wanted = Some((
            crate::core_terminals::Purpose::Tab {
                scope: scope.clone(),
                workspace: "old".into(),
            },
            herdr_backend::Spawn::default(),
        ));
        view.core_terminals.starting = Some((
            crate::core_terminals::Purpose::Tab {
                scope,
                workspace: "old".into(),
            },
            pending,
        ));
        view.retire_core_connection(cx);
        assert!(view.core_terminals.wanted.is_none());
        assert!(view.core_terminals.starting.is_none());
        assert_eq!(view.core_workspace_key(), None);
        assert!(matches!(
            view.core_terminals.hub.state(),
            crate::core_terminals::hub::State::Idle
        ));
        assert!(late.send(Err(herdr_core_client::Error::Lost)).is_err());
    });
}
