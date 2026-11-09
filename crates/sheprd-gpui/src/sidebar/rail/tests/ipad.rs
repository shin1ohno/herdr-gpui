use super::*;
use crate::ipad_ui::metrics::*;

#[gpui::test]
fn ipad_rail_uses_touch_buttons_and_themed_marks(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = rail_window(cx, 1);
    view.update(cx, |view, cx| {
        view.live.status = crate::state::ConnectionStatus::Connected;
        cx.notify();
    });
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    assert_eq!(cx.debug_bounds("sidebar-rail").unwrap().size.width, px(72.));
    for selector in [
        "rail-host-local",
        "rail-workspace-local-w0",
        "rail-agent-local-p0",
        "rail-new-workspace",
        "rail-expand",
    ] {
        assert_eq!(
            cx.debug_bounds(selector).unwrap().size,
            size(px(64.), px(56.)),
            "{selector}"
        );
    }
    assert_eq!(
        cx.debug_bounds("rail-dot-rail-workspace-local-w0")
            .unwrap()
            .size,
        size(px(12.), px(12.))
    );
    assert_eq!(
        cx.debug_bounds("rail-agent-icon-local-p0").unwrap().size,
        size(px(20.), px(20.))
    );
    assert_eq!(
        cx.debug_bounds("rail-dot-rail-agent-local-p0")
            .unwrap()
            .size,
        size(px(10.), px(10.))
    );
    assert_eq!(
        cx.debug_bounds("rail-agent-ring-local-p0").unwrap().size,
        size(px(14.), px(14.))
    );
    assert_eq!(
        cx.debug_bounds("ipad-rail-divider").unwrap().size,
        size(px(40.), px(1.))
    );
    let ring = cx.debug_bounds("rail-agent-ring-local-p0").unwrap();
    let dot = cx.debug_bounds("rail-dot-rail-agent-local-p0").unwrap();
    assert_eq!(dot.origin - ring.origin, point(px(2.), px(2.)));
    assert_eq!(SidebarMode::Rail.width(None, 1194.), Some(RAIL));
    assert_eq!(SidebarMode::Rail.width(None, 270.), Some(30.));
    let expand = cx.debug_bounds("rail-expand").unwrap();
    cx.simulate_click(expand.center(), Modifiers::default());
    view.read_with(cx, |view, _| assert!(view.sidebar_visible));
}

#[gpui::test]
fn ipad_rail_badges_count_only_blocked_agents_of_the_space(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = rail_window(cx, 1);
    view.update(cx, |view, cx| {
        let snapshot = Arc::make_mut(view.live.snapshot.as_mut().unwrap());
        snapshot.agents[0].agent_status = AgentStatus::Blocked;
        snapshot.agents[1].agent_status = AgentStatus::Done;
        let mut blocked = snapshot.agents[0].clone();
        blocked.pane_id = "another-blocked-pane".into();
        snapshot.agents.push(blocked);
        cx.notify();
    });
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    let badge = cx
        .debug_bounds("rail-badge-rail-workspace-local-w0")
        .unwrap();
    assert_eq!(badge.size, size(px(16.), px(16.)));
    let row = cx.debug_bounds("rail-workspace-local-w0").unwrap();
    assert!(badge.center().x > row.center().x && badge.center().y < row.center().y);
    assert!(
        cx.debug_bounds("rail-badge-rail-workspace-local-w1")
            .is_none()
    );
}

#[gpui::test]
fn ipad_right_click_belongs_only_to_a_menu_that_opened(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = rail_window(cx, 1);
    let selector = "rail-workspace-local-w0";
    let position = cx.debug_bounds(selector).unwrap().center();
    // A stale painted row must not claim the right press after its workspace
    // has disappeared from the current snapshot.
    view.update(cx, |view, _| {
        Arc::make_mut(view.live.snapshot.as_mut().unwrap())
            .workspaces
            .clear();
    });
    cx.simulate_mouse_down(position, MouseButton::Right, Modifiers::default());
    view.read_with(cx, |view, _| {
        assert!(view.menu.page.is_none());
        assert!(!view.menu.opening_right_click);
    });
    cx.simulate_mouse_up(position, MouseButton::Right, Modifiers::default());
}

#[test]
fn ipad_rail_labels_ellipsis_and_expand_prose_translate() {
    use crate::i18n::leak_tests::{assert_translated, draw, uncut, under_en_xa};
    under_en_xa("iPad rail", |cx, recorder| {
        let _ipad = crate::ipad_ui::ipad_layout();
        let (view, cx) = rail_window(cx, 1);
        let mut seen = recorder.take();
        view.update(cx, |view, cx| {
            let snapshot = Arc::make_mut(view.live.snapshot.as_mut().unwrap());
            snapshot.agents[0].agent_status = AgentStatus::Blocked;
            let mut second = snapshot.agents[0].clone();
            second.pane_id = "second-blocked".into();
            snapshot.agents.push(second);
            cx.notify();
        });
        draw(cx);
        let badge_frame = recorder.take();
        assert!(
            badge_frame.iter().any(|line| line == "2"),
            "{badge_frame:?}"
        );
        seen.extend(badge_frame);
        let mut data = view.read_with(cx, |view, _| {
            let snapshot = view.live.snapshot.as_deref().unwrap();
            let mut data = vec![view.endpoints[0].label.clone()];
            data.extend(
                snapshot
                    .workspaces
                    .iter()
                    .map(|workspace| workspace_label(workspace, false).to_owned()),
            );
            data.extend(
                snapshot
                    .workspaces
                    .iter()
                    .filter_map(|workspace| workspace.branch.clone()),
            );
            data.extend(
                snapshot
                    .agents
                    .iter()
                    .map(|agent| agent_label(agent, snapshot)),
            );
            for command in [
                Command::Tab,
                Command::SplitRight,
                Command::ToggleSidebar,
                Command::Palette,
                Command::ClosePane,
                Command::Keybinds,
            ] {
                data.extend(view.shortcut_hint(command));
            }
            data.push(env!("HERDR_BUILD_BRANCH").to_owned());
            data
        });
        data.push("sidebar-child".into());
        data.push("sidebar-child-with-a-long-readable-branch-name".into());
        let cuts: Vec<_> = seen
            .iter()
            .filter(|line| {
                line.strip_suffix('…')
                    .is_some_and(|stem| data.iter().any(|name| name.starts_with(stem)))
            })
            .cloned()
            .collect();
        data.extend(cuts);
        assert!(seen.iter().any(|line| line.ends_with('…')), "{seen:?}");
        let label = cx
            .debug_bounds("rail-label-rail-workspace-local-w1")
            .unwrap();
        assert!(label.size.width <= px(RAIL_BUTTON_W));
        assert_translated("iPad rail", &uncut(seen.clone()), &data);
        view.update(cx, |view, cx| {
            view.sidebar_visible = false;
            cx.notify();
        });
        draw(cx);
        seen.extend(recorder.take());
        assert_translated("iPad rail redraw", &uncut(seen), &data);
    });
}
