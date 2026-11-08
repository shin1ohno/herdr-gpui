use super::*;
use crate::{browser::Store, state::ConnectionStatus};
use sheprd_backend::Generation;

#[gpui::test]
fn saved_core_host_picker_ignores_a_stale_herdr1_connecting_projection(
    cx: &mut gpui::TestAppContext,
) {
    const PROFILE: &str = "0123456789abcdef0123456789abcdef";
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update(cx, |view, cx| {
        Store::default().install(cx);
        view.config
            .devices
            .entry(PROFILE.into())
            .or_default()
            .generation = Some(Generation::Herdr2);
        // Match the catalog's settings snapshot after an explicit Herdr 2 registration.
        view.catalog.fixture_generation(PROFILE, Generation::Herdr2);
        view.reconcile_catalog(
            vec![sheprd_client::SavedHost {
                id: PROFILE.into(),
                label: "Core host".into(),
                target: "user@core.invalid".into(),
                session: "default".into(),
                enabled: true,
            }],
            Vec::new(),
            cx,
        );
        view.selected_endpoint = 1;
        view.endpoints[0].live.status = ConnectionStatus::Disconnected;
        view.endpoints[1].live.status = ConnectionStatus::Connecting;
        view.live.status = ConnectionStatus::Connecting;
        if cfg!(target_os = "ios") {
            assert_eq!(view.endpoints[1].backend_generation(), Generation::Herdr2);
            assert_eq!(
                view.picker_connection_text(&view.endpoints[1]),
                ConnectionStatus::Disconnected.label()
            );
            assert!(!view.sessions_available());
        } else {
            assert_eq!(
                view.picker_connection_text(&view.endpoints[1]),
                "connecting"
            );
        }
        assert_eq!(view.connected_picker_devices(), 0);

        view.detach_core_connection(cx);
        if cfg!(target_os = "ios") {
            assert_eq!(
                view.picker_connection_text(&view.endpoints[1]),
                ConnectionStatus::Detached.label()
            );
        }
        assert_eq!(view.connected_picker_devices(), 0);
        assert_eq!(view.live.status, ConnectionStatus::Connecting);
    });
}
