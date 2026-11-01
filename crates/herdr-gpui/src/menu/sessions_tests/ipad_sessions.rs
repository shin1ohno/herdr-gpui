//! iPad's Sessions entry on the device picker: the chip that unfolds a device
//! sits only on a Herdr 1 host over SSH, which is where the shortcut opens.
use super::*;
use herdr_backend::Generation;

#[gpui::test]
fn ipad_sessions_controls_follow_the_selected_remote_device(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(800.), px(600.)));
    draw(cx);

    view.update_in(cx, |view, window, cx| {
        // This device has no daemon on iPad, so there is nothing to open.
        view.open_sessions(gpui::point(px(10.), px(550.)), window, cx);
        assert_eq!(
            view.menu.page == Some(Page::Devices),
            !cfg!(target_os = "ios")
        );
        view.dismiss_menu(window, cx);
        view.endpoints[0].id = LOCAL.into();
        // Device rows 2 and 3: a Herdr 1 host and a Herdr 2 one.
        view.endpoints.push(remote(
            "ssh:ipad-session",
            "Remote",
            "user@session.invalid",
            "work",
        ));
        let mut core = remote("ssh:ipad-core", "Core", "user@core.invalid", "default");
        core.set_backend_generation(Generation::Herdr2);
        view.endpoints.push(core);
        view.selected_endpoint = 1;
        view.open_sessions(gpui::point(px(10.), px(550.)), window, cx);
        assert_eq!(view.menu.page, Some(Page::Devices));
        assert_eq!(
            view.menu.unfolded_device.as_deref(),
            Some("ssh:ipad-session")
        );
    });
    draw(cx);
    assert!(cx.debug_bounds("device-sessions-2").is_some());
    assert!(cx.debug_bounds("sessions-row-0").is_some());
    // iPad lists no local device, and a Herdr 2 host has no session list.
    assert_eq!(
        cx.debug_bounds("device-row-1").is_some(),
        !cfg!(target_os = "ios")
    );
    assert_eq!(
        cx.debug_bounds("device-sessions-3").is_some(),
        !cfg!(target_os = "ios")
    );
}
