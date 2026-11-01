use super::*;
use crate::{endpoint::Endpoint, state::ConnectionStatus};
use gpui::{KeyDownEvent, Keystroke};
use herdr_client::ConnectTarget;

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
        let listed: Vec<_> = view
            .picker_endpoints()
            .map(|endpoint| endpoint.id.as_str())
            .collect();
        assert!(listed.contains(&"ssh:first"));
        assert!(listed.contains(&"ssh:second"));
        assert_eq!(
            listed.contains(&crate::endpoint::LOCAL),
            !cfg!(target_os = "ios")
        );
        assert_eq!(view.add_device_row(), listed.len() + 1);
        let first = if cfg!(target_os = "ios") {
            "ssh:first"
        } else {
            crate::endpoint::LOCAL
        };
        assert_eq!(view.picker_endpoints().next().unwrap().id, first);

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
        let last = if view.sessions_available() {
            view.session_list_row()
        } else {
            view.add_device_row()
        };
        assert_eq!(view.menu.selected, Some(last));
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
