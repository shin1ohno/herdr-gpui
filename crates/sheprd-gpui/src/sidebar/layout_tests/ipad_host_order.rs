use super::*;

fn remote_hosts(window: &mut Window, cx: &mut Context<SheprdWindow>) -> SheprdWindow {
    let mut view = fixture_window(window, cx);
    view.config.show_agents = false;
    view.config.sidebar_style.activity_bar = false;
    view.live.snapshot = None;
    for id in ["ssh:pro", "ssh:mini", "ssh:cloud"] {
        let mut endpoint = crate::endpoint::Endpoint::new(
            id.into(),
            id.into(),
            ConnectTarget::Ssh {
                target: "unused".into(),
                session: "default".into(),
            },
            true,
        );
        endpoint.collapsed = true;
        view.endpoints.push(endpoint);
    }
    view.selected_endpoint = 1;
    view.device_filter = crate::menu::DeviceScope::Only(
        ["ssh:pro", "ssh:mini", "ssh:cloud"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
    );
    view
}

fn heading_precedes_hosts(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(remote_hosts);
    cx.simulate_resize(size(px(1194.), px(834.)));
    for searchable in [false, true] {
        view.update(cx, |view, cx| {
            view.config.show_sidebar_search = searchable;
            cx.notify();
        });
        cx.update(|window, cx| full_draw(window, cx).clear(cx));
        let heading = cx.debug_bounds("header-spaces").unwrap();
        let mut bottom = heading.bottom();
        for (id, selector) in [
            ("ssh:pro", "host-ssh:pro"),
            ("ssh:mini", "host-ssh:mini"),
            ("ssh:cloud", "host-ssh:cloud"),
        ] {
            let host = cx.debug_bounds(selector).unwrap();
            assert!(
                bottom <= host.top(),
                "{id} must follow the heading and prior host"
            );
            bottom = host.bottom();
        }
        assert_eq!(
            cx.debug_bounds("sidebar-search").is_some(),
            searchable && !crate::ipad_ui::layout()
        );
        assert_eq!(
            cx.debug_bounds("sidebar-search-open").is_some(),
            searchable && crate::ipad_ui::layout()
        );
        view.read_with(cx, |view, _| {
            assert_eq!(view.endpoints[view.selected_endpoint].id, "ssh:pro");
            assert!(
                view.endpoints[1..]
                    .iter()
                    .all(|endpoint| endpoint.collapsed)
            );
        });
    }
}

#[gpui::test]
fn ipad_spaces_heading_precedes_three_remote_hosts(cx: &mut gpui::TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    heading_precedes_hosts(cx);
}

#[gpui::test]
fn desktop_keeps_spaces_heading_before_hosts(cx: &mut gpui::TestAppContext) {
    let _desktop = crate::ipad_ui::desktop();
    heading_precedes_hosts(cx);
}

#[gpui::test]
fn ipad_keeps_heading_when_no_host_is_visible(cx: &mut gpui::TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = remote_hosts(window, cx);
        // A stale scope can outlive a removed catalog entry.
        view.device_filter = crate::menu::DeviceScope::only("ssh:removed");
        view
    });
    cx.simulate_resize(size(px(1194.), px(834.)));
    for searchable in [false, true] {
        view.update(cx, |view, cx| {
            view.config.show_sidebar_search = searchable;
            cx.notify();
        });
        cx.update(|window, cx| full_draw(window, cx).clear(cx));
        let heading = cx.debug_bounds("header-spaces").unwrap();
        let list = cx.debug_bounds("spaces-scroll").unwrap();
        assert!(heading.bottom() <= list.top());
        assert_eq!(
            cx.debug_bounds("sidebar-search").is_some(),
            searchable && !crate::ipad_ui::layout()
        );
        assert_eq!(
            cx.debug_bounds("sidebar-search-open").is_some(),
            searchable && crate::ipad_ui::layout()
        );
        assert!(cx.debug_bounds("host-ssh:pro").is_none());
    }
}
