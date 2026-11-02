use super::*;
use crate::{HerdrWindow, config::AgentsPlacement};
use gpui::{Bounds, Context, Modifiers, TestAppContext, VisualContext, Window, point, px, size};
use std::sync::Arc;

fn fixture(window: &mut Window, cx: &mut Context<HerdrWindow>) -> HerdrWindow {
    let mut view = layout_tests::fixture_window(window, cx);
    let mut snapshot = layout_tests::snapshot(6);
    snapshot.agents[0].agent_status = AgentStatus::Blocked;
    snapshot.workspaces[0].agent_status = AgentStatus::Blocked;
    view.live.snapshot = Some(Arc::new(snapshot));
    view.live.status = crate::state::ConnectionStatus::Connected;
    view.config.agents_placement = AgentsPlacement::Panel;
    view.config.show_agents = true;
    view.config.show_sidebar_search = false;
    view.config.sidebar_style.activity_bar = false;
    view.sidebar_split = Some(0.72);
    view
}

#[gpui::test]
fn ipad_rows_have_fixed_touch_geometry_and_worktree_indent(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(fixture);
    cx.simulate_resize(size(px(1194.), px(834.)));
    cx.update(|window, cx| layout_tests::full_draw(window, cx).clear(cx));
    for (selector, height) in [
        ("host-local", 64.),
        ("row-herdr", 56.),
        ("row-agent-p0", 44.),
        ("new-workspace", 44.),
        ("ipad-sidebar-settings", 44.),
        ("collapse-host-local", 44.),
    ] {
        assert_eq!(
            cx.debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing {selector}"))
                .size
                .height,
            px(height),
            "{selector}"
        );
    }
    let parent = cx.debug_bounds("ipad-space-name-w3").unwrap();
    assert!(
        cx.debug_bounds("host-local").unwrap().bottom()
            <= cx.debug_bounds("header-spaces").unwrap().top()
    );
    let child = cx.debug_bounds("ipad-space-name-w4").unwrap();
    assert_eq!(child.left() - parent.left(), px(22.));
    let count = cx.debug_bounds("ipad-tab-count-w0").unwrap();
    let badge = cx.debug_bounds("ipad-needs-you-w0").unwrap();
    assert!(count.right() <= badge.left());
    assert_eq!(count.center().y, badge.center().y);
    assert!(badge.right() <= cx.debug_bounds("row-herdr").unwrap().right());
    let name_left = cx.debug_bounds("ipad-space-name-w0").unwrap().left();
    let meta_right = cx.debug_bounds("ipad-agent-meta-agent-p0").unwrap().right();
    view.update(cx, |view, cx| {
        let snapshot = Arc::make_mut(view.live.snapshot.as_mut().unwrap());
        snapshot.workspaces[0].focused = false;
        snapshot.agents[0].focused = true;
        cx.notify();
    });
    cx.update(|window, cx| layout_tests::full_draw(window, cx).clear(cx));
    assert_eq!(
        cx.debug_bounds("ipad-space-name-w0").unwrap().left(),
        name_left
    );
    assert_eq!(
        cx.debug_bounds("ipad-agent-meta-agent-p0").unwrap().right(),
        meta_right
    );
}

#[gpui::test]
fn ipad_long_branches_remain_inside_rows_at_saved_narrow_widths(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(fixture);
    cx.simulate_resize(size(px(1194.), px(834.)));
    for width in [300., 240., 180.] {
        view.update(cx, |view, cx| {
            view.sidebar_width = Some(width);
            cx.notify();
        });
        cx.update(|window, cx| layout_tests::full_draw(window, cx).clear(cx));
        let branch = cx.debug_bounds("ipad-branch-w1").unwrap();
        let row = cx
            .debug_bounds("row-herdr-gpui-sidebar-rendering-regression-investigation")
            .unwrap();
        assert!(
            branch.right() <= row.right(),
            "{width}: {branch:?} outside {row:?}"
        );
        assert!(branch.size.width < px(400.));
        assert_eq!(row.size.height, px(56.));
    }
}

#[gpui::test]
fn ipad_heading_and_footer_chips_need_hardware_and_preserve_search(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture(window, cx);
        view.config.show_sidebar_search = true;
        view
    });
    cx.simulate_resize(size(px(1194.), px(834.)));
    cx.update(|window, cx| layout_tests::full_draw(window, cx).clear(cx));
    assert!(cx.debug_bounds("sidebar-search").is_some());
    assert!(cx.debug_bounds("ipad-spaces-shortcut").is_none());
    cx.simulate_keystrokes("escape");
    cx.update(|window, cx| layout_tests::full_draw(window, cx).clear(cx));
    assert!(cx.debug_bounds("ipad-spaces-shortcut").is_some());
    assert!(cx.debug_bounds("ipad-new-space-shortcut").is_some());
    view.update(cx, |view, _| {
        view.config.shortcut_style = crate::config::ShortcutStyle::HerdrPrefix;
        assert_eq!(
            view.shortcut_hint(crate::Command::WorkspacePicker)
                .as_deref(),
            Some("⌃B w")
        );
    });
    cx.simulate_window_visual_viewport_change(
        cx.window_handle(),
        Bounds::new(point(px(0.), px(0.)), size(px(1194.), px(534.))),
    );
    cx.update(|window, cx| layout_tests::full_draw(window, cx).clear(cx));
    assert!(cx.debug_bounds("ipad-spaces-shortcut").is_none());
    assert!(cx.debug_bounds("ipad-new-space-shortcut").is_none());
}

#[gpui::test]
fn ipad_workspace_popover_has_no_detach_and_host_collapse_still_works(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(fixture);
    cx.simulate_resize(size(px(1194.), px(834.)));
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_workspace_menu("w0", point(px(150.), px(180.)), window, cx)
        });
        layout_tests::full_draw(window, cx).clear(cx);
    });
    assert!(cx.debug_bounds("workspace-menu-Detach").is_none());
    assert!(cx.debug_bounds("workspace-menu-detach").is_none());
    assert!(cx.debug_bounds("workspace-menu-Rename").is_some());
    view.update_in(cx, |view, window, cx| view.dismiss_menu(window, cx));
    cx.update(|window, cx| layout_tests::full_draw(window, cx).clear(cx));
    let collapse = cx.debug_bounds("collapse-host-local").unwrap().center();
    cx.simulate_click(collapse, Modifiers::default());
    cx.update(|window, cx| layout_tests::full_draw(window, cx).clear(cx));
    view.read_with(cx, |view, _| assert!(view.endpoints[0].collapsed));
    assert!(cx.debug_bounds("row-herdr").is_none());
}

#[gpui::test]
fn ipad_each_device_keeps_a_host_header_including_a_core_only_host(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (_view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture(window, cx);
        let mut core = crate::endpoint::Endpoint::new(
            "ssh:core".into(),
            "Core only".into(),
            herdr_client::ConnectTarget::Ssh {
                target: "me@core.test".into(),
                session: "default".into(),
            },
            true,
        );
        core.set_backend_generation(herdr_backend::Generation::Herdr2);
        view.endpoints.push(core);
        view
    });
    cx.simulate_resize(size(px(1194.), px(834.)));
    cx.update(|window, cx| layout_tests::full_draw(window, cx).clear(cx));
    for selector in ["host-local", "host-ssh:core"] {
        assert_eq!(cx.debug_bounds(selector).unwrap().size.height, px(64.));
    }
}
