use super::*;
use crate::endpoint::Endpoint;
use sheprd_client::ConnectTarget;

fn remote() -> Endpoint {
    Endpoint::new(
        "ssh:picker".into(),
        "Remote".into(),
        ConnectTarget::Ssh {
            target: "user@picker.invalid".into(),
            session: "default".into(),
        },
        true,
    )
}

#[gpui::test]
fn picker_projection_retains_disabled_device_text(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, _| {
        view.endpoints.push(remote());
        view.selected_endpoint = view.endpoints.len() - 1;
        view.endpoints[view.selected_endpoint].enabled = false;
        assert_eq!(
            view.picker_connection_text(&view.endpoints[view.selected_endpoint]),
            "disabled"
        );
        assert_eq!(view.sessions_available(), !cfg!(target_os = "ios"));
    });
}
