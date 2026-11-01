use super::*;

#[gpui::test]
fn ipad_sessions_controls_follow_the_selected_remote_device(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(800.), px(600.)));
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    assert_eq!(
        cx.debug_bounds("device-sessions").is_some(),
        !cfg!(target_os = "ios")
    );

    view.update_in(cx, |view, window, cx| {
        view.open_sessions(gpui::point(px(10.), px(550.)), window, cx);
        assert_eq!(
            view.menu.page == Some(Page::Sessions),
            !cfg!(target_os = "ios")
        );
        view.dismiss_menu(window, cx);
        view.endpoints.push(remote(
            "ssh:ipad-session",
            "Remote",
            "user@session.invalid",
            "work",
        ));
        view.selected_endpoint = view.endpoints.len() - 1;
        view.open_sessions(gpui::point(px(10.), px(550.)), window, cx);
        assert_eq!(view.menu.page, Some(Page::Sessions));
    });
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    assert!(cx.debug_bounds("device-sessions").is_some());
    assert!(cx.debug_bounds("sessions-current-device").is_some());
    assert_eq!(
        cx.debug_bounds("sessions-local-header").is_some(),
        !cfg!(target_os = "ios")
    );
    assert_eq!(
        cx.debug_bounds("sessions-empty").is_some(),
        !cfg!(target_os = "ios")
    );
}
