use super::*;

#[gpui::test]
fn an_empty_catalog_keeps_add_device_and_settings_in_reach(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        view.endpoints[0].connection.target = ConnectTarget::Local;
        view.endpoints[0].enabled = false;
        view.live.snapshot = None;
        view
    });
    cx.simulate_resize(size(px(684.), px(912.)));
    for bar in [false, true] {
        cx.update(|window, cx| {
            view.update(cx, |view, _| view.config.sidebar_style.activity_bar = bar);
            full_draw(window, cx).clear(cx);
        });
        for selector in [
            "device-picker",
            if bar {
                "activity-settings"
            } else {
                "device-settings"
            },
        ] {
            let bounds = cx.debug_bounds(selector).unwrap();
            assert!(bounds.top() >= px(0.) && bounds.bottom() <= px(912.));
        }
    }
    let picker = cx.debug_bounds("device-picker").unwrap();
    cx.simulate_click(picker.center(), Modifiers::default());
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    view.read_with(cx, |view, _| {
        assert_eq!(view.menu.page, Some(crate::menu::Page::Devices));
        assert!(view.device_setup_unavailable().is_none());
    });
    let add = cx.debug_bounds("device-add").unwrap();
    assert!(add.top() >= px(0.) && add.bottom() < picker.top());
}
