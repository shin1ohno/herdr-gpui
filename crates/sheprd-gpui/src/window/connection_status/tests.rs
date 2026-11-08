use crate::{endpoint::Endpoint, sidebar::layout_tests::fixture_window, state::ConnectionStatus};
use sheprd_client::ConnectTarget;

#[gpui::test]
fn selected_and_background_devices_use_their_own_live_projection(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, _| {
        view.live.status = ConnectionStatus::Connected;
        view.endpoints[0].live.status = ConnectionStatus::Disconnected;
        let mut background = Endpoint::new(
            "ssh:background".into(),
            "Background".into(),
            ConnectTarget::Ssh {
                target: "user@background".into(),
                session: "default".into(),
            },
            true,
        );
        background.live.status = ConnectionStatus::Connecting;
        view.endpoints.push(background);
        assert_eq!(
            view.displayed_connection_status(&view.endpoints[0]),
            ConnectionStatus::Connected
        );
        assert_eq!(
            view.displayed_connection_status(&view.endpoints[1]),
            ConnectionStatus::Connecting
        );
        view.live.status = ConnectionStatus::Detached;
        assert_eq!(
            view.displayed_connection_status(&view.endpoints[0]),
            ConnectionStatus::Detached
        );
    });
}
