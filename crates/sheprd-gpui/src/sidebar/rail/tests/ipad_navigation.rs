use super::*;

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
}

#[gpui::test]
fn folded_ipad_hosts_reveal_their_spaces_and_agents_when_selected(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = rail_window(cx, 3);
    view.update(cx, |view, cx| {
        view.device_filter =
            crate::menu::DeviceScope::Only(["ssh:host1", "ssh:host2"].map(str::to_owned).into());
        for endpoint in &mut view.endpoints[1..] {
            endpoint.collapsed = true;
        }
        view.selected_endpoint = 1;
        view.live = view.endpoints[1].live.clone();
        cx.notify();
    });
    draw(cx);
    assert!(cx.debug_bounds("rail-host-ssh:host1").is_some());
    assert!(cx.debug_bounds("rail-host-ssh:host2").is_some());
    assert!(
        cx.debug_bounds("rail-host-ssh:host1").unwrap().top()
            < cx.debug_bounds("rail-host-ssh:host2").unwrap().top()
    );
    assert!(cx.debug_bounds("rail-workspace-ssh:host1-w0").is_none());
    assert!(cx.debug_bounds("rail-agent-ssh:host1-p0").is_none());
    let host = cx.debug_bounds("rail-host-ssh:host2").unwrap();
    cx.simulate_click(host.center(), Modifiers::default());
    draw(cx);
    assert!(cx.debug_bounds("rail-workspace-ssh:host2-w0").is_some());
    assert!(cx.debug_bounds("rail-agent-ssh:host2-p0").is_some());
    view.read_with(cx, |view, _| {
        assert_eq!(view.selected_endpoint, 2);
        assert!(view.endpoints[1].collapsed, "other host stays folded");
        assert!(!view.endpoints[2].collapsed);
        assert!(!view.sidebar_visible);
        assert_eq!(
            view.live.snapshot.as_ref().unwrap().workspaces.len(),
            view.endpoints[2]
                .live
                .snapshot
                .as_ref()
                .unwrap()
                .workspaces
                .len()
        );
    });
    assert_eq!(
        cx.debug_bounds("rail-dot-rail-agent-ssh:host2-p0")
            .unwrap()
            .size,
        size(px(10.), px(10.))
    );
    let workspace = cx.debug_bounds("rail-workspace-ssh:host2-w0").unwrap();
    cx.simulate_click(workspace.center(), Modifiers::default());
    view.read_with(cx, |view, _| {
        assert_eq!(
            view.pending_navigation,
            Some(NavigationTarget::Workspace("w0".into()))
        );
    });
    draw(cx);
    let agent = cx.debug_bounds("rail-agent-ssh:host2-p0").unwrap();
    cx.simulate_click(agent.center(), Modifiers::default());
    view.read_with(cx, |view, _| {
        assert_eq!(
            view.pending_navigation,
            Some(NavigationTarget::Pane("p0".into()))
        );
    });
    // Opening the already selected folded host must work as well.
    view.update(cx, |view, cx| {
        view.endpoints[2].collapsed = true;
        cx.notify();
    });
    draw(cx);
    let host = cx.debug_bounds("rail-host-ssh:host2").unwrap();
    cx.simulate_click(host.center(), Modifiers::default());
    draw(cx);
    assert!(cx.debug_bounds("rail-agent-ssh:host2-p0").is_some());
    let expand = cx.debug_bounds("rail-expand").unwrap();
    cx.simulate_click(expand.center(), Modifiers::default());
    view.read_with(cx, |view, _| {
        assert!(view.sidebar_visible);
        assert_eq!(view.selected_endpoint, 2);
        assert!(!view.endpoints[2].collapsed);
        assert!(view.endpoints[1].collapsed);
    });
}

#[gpui::test]
fn essential_ipad_rail_controls_survive_hidden_agents_and_filtered_devices(
    cx: &mut TestAppContext,
) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = rail_window(cx, 3);
    // Devices opens a workspace tab. The geometry-only rail fixture has no
    // focused workspace until a connected daemon snapshot supplies one.
    view.update(cx, |view, cx| {
        let mut shown = snapshot(6);
        shown.focused_workspace_id = Some("w0".into());
        shown.focused_tab_id = Some("t0".into());
        view.live.snapshot = Some(Arc::new(shown));
        view.endpoints[0].live = view.live.clone();
        cx.notify();
    });
    for width in [560., 1194.] {
        cx.simulate_resize(size(px(width), px(834.)));
        view.update(cx, |view, cx| {
            view.config.show_agents = false;
            view.config.sidebar_style.activity_bar = false;
            view.device_filter = crate::menu::DeviceScope::only("local");
            cx.notify();
        });
        draw(cx);
        let rail = cx.debug_bounds("sidebar-rail").unwrap();
        assert!(cx.debug_bounds("rail-agents").is_none());
        assert!(cx.debug_bounds("rail-host-ssh:host1").is_none());
        for selector in ["rail-devices", "rail-settings"] {
            let button = cx.debug_bounds(selector).unwrap();
            assert_eq!(button.size, size(px(64.), px(44.)));
            assert!(button.top() >= rail.top() && button.bottom() <= rail.bottom());
            assert!(button.left() >= rail.left() && button.right() <= rail.right());
        }
        view.read_with(cx, |view, _| {
            assert!(!view.config.show_agents);
            assert_eq!(view.endpoints[1].id, "ssh:host1");
            assert_eq!(view.endpoints[2].id, "ssh:host2");
        });
    }
    let devices = cx.debug_bounds("rail-devices").unwrap();
    cx.simulate_click(devices.center(), Modifiers::default());
    draw(cx);
    view.read_with(cx, |view, _| assert!(view.devices_overview_open()));
}
