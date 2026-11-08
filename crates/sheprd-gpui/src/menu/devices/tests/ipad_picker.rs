use super::*;
use crate::menu::devices::{lists_sessions, picker::Line, picker_lists};
use crate::{endpoint::Endpoint, state::ConnectionStatus};
use gpui::{KeyDownEvent, Keystroke};
use sheprd_backend::Generation;
use sheprd_client::ConnectTarget;

fn remote(id: &str) -> Endpoint {
    Endpoint::new(
        id.into(),
        id.into(),
        ConnectTarget::Ssh {
            target: "user@unused.invalid".into(),
            session: "default".into(),
        },
        true,
    )
}

#[gpui::test]
fn picker_navigation_addresses_only_the_devices_it_lists(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update_in(cx, |view, window, cx| {
        view.endpoints[0].id = crate::endpoint::LOCAL.into();
        view.endpoints.push(remote("ssh:first"));
        view.endpoints.push(remote("ssh:second"));
        let listed: Vec<String> = view
            .picker_lines()
            .iter()
            .filter_map(|line| match line {
                Line::Device(index) => Some(view.endpoints[*index].id.clone()),
                _ => None,
            })
            .collect();
        let projected: Vec<String> = view
            .picker_endpoints()
            .map(|endpoint| endpoint.id.clone())
            .collect();
        assert_eq!(listed, projected);
        assert!(listed.iter().any(|id| id == "ssh:first"));
        assert!(listed.iter().any(|id| id == "ssh:second"));
        assert_eq!(
            listed.iter().any(|id| id == crate::endpoint::LOCAL),
            !cfg!(target_os = "ios")
        );
        let first = if cfg!(target_os = "ios") {
            "ssh:first"
        } else {
            crate::endpoint::LOCAL
        };
        assert_eq!(listed[0], first);

        view.menu.page = Some(Page::Devices);
        view.menu.selected = Some(0);
        view.devices_key(
            &KeyDownEvent {
                keystroke: Keystroke::parse("up").unwrap(),
                is_held: false,
                prefer_character_input: false,
            },
            window,
            cx,
        );
        // Up from All Devices wraps to Add Device, the last line.
        let last = view.picker_ordinal(|line| matches!(line, Line::AddDevice));
        assert_eq!(view.menu.selected, last);
    });
}

#[gpui::test]
fn picker_connected_count_uses_the_same_projection_as_the_sidebar(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, _| {
        // The selected endpoint's stored projection can lag the view's live one.
        view.endpoints[0].id = "ssh:selected".into();
        view.endpoints[0].connection.target = ConnectTarget::Ssh {
            target: "user@selected.invalid".into(),
            session: "default".into(),
        };
        view.endpoints[0].live.status = ConnectionStatus::Disconnected;
        view.live.status = ConnectionStatus::Connected;
        view.endpoints.push(remote("ssh:background"));
        view.endpoints[1].live.status = ConnectionStatus::Connected;
        assert_eq!(view.connected_picker_devices(), 2);
        view.live.status = ConnectionStatus::Detached;
        assert_eq!(view.connected_picker_devices(), 1);
        view.endpoints[1].live.status = ConnectionStatus::Disconnected;
        assert_eq!(view.connected_picker_devices(), 0);
    });
}

#[gpui::test]
fn ipad_session_control_requires_an_enabled_remote_device(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, _| {
        assert_eq!(view.sessions_available(), !cfg!(target_os = "ios"));
        view.endpoints.push(remote("ssh:remote"));
        view.selected_endpoint = 1;
        assert!(view.sessions_available());
        view.endpoints[1].enabled = false;
        assert_eq!(view.sessions_available(), !cfg!(target_os = "ios"));
    });
}

#[test]
fn ipad_unfolds_only_enabled_herdr1_hosts_over_ssh() {
    let mut host = remote("ssh:herdr1");
    assert!(lists_sessions(true, &host));
    host.enabled = false;
    assert!(!lists_sessions(true, &host));
    assert!(lists_sessions(false, &host));

    let mut core = remote("ssh:core");
    core.set_backend_generation(Generation::Herdr2);
    assert!(!lists_sessions(true, &core));
    assert!(lists_sessions(false, &core));

    let local = Endpoint::new(
        crate::endpoint::LOCAL.into(),
        "This device".into(),
        ConnectTarget::Local,
        true,
    );
    assert!(!lists_sessions(true, &local));
    assert!(lists_sessions(false, &local));
}

#[test]
fn ipad_picker_lists_saved_hosts_but_never_the_local_sentinel() {
    let local = Endpoint::new(
        crate::endpoint::LOCAL.into(),
        "This device".into(),
        ConnectTarget::Local,
        true,
    );
    assert!(!picker_lists(true, &local));
    assert!(picker_lists(false, &local));

    let mut core = remote("ssh:core");
    core.set_backend_generation(Generation::Herdr2);
    for endpoint in [remote("ssh:herdr1"), core] {
        assert!(picker_lists(true, &endpoint));
        assert!(picker_lists(false, &endpoint));
    }
}

#[gpui::test]
fn ipad_unfolding_a_device_without_sessions_lists_nothing(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, _| {
        let mut core = remote("ssh:core");
        core.set_backend_generation(Generation::Herdr2);
        view.endpoints.push(core);
        view.menu.page = Some(Page::Devices);
        view.unfold_device(1);
        let unfolded = view.menu.unfolded_device.is_some();
        let sessions = view
            .picker_lines()
            .iter()
            .any(|line| matches!(line, Line::Session(_)));
        assert_eq!(unfolded, !cfg!(target_os = "ios"));
        assert_eq!(sessions, !cfg!(target_os = "ios"));
    });
}
