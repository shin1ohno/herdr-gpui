use super::*;
use crate::{menu::Page, state::ConnectionStatus};
use gpui::{InputEvent, LongPressEvent, TouchPhase};

#[gpui::test]
fn rail_agents_offer_their_pane_menu_to_touch_and_pointer(cx: &mut TestAppContext) {
    let (view, cx) = rail_window(cx, 1);
    view.update(cx, |view, cx| {
        let mut listing = snapshot(1);
        listing.agents.truncate(1);
        listing.panes = vec![
            serde_json::from_value(serde_json::json!({
                "pane_id": "p0", "workspace_id": "w0", "tab_id": "t0", "label": "review",
                "cwd": "/repo", "foreground_cwd": "/repo", "focused": false,
                "right_click_passthrough": false,
            }))
            .unwrap(),
        ];
        view.live.snapshot = Some(Arc::new(listing));
        view.live.status = ConnectionStatus::Connected;
        cx.notify();
    });
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    let position = cx.debug_bounds("rail-agent-local-p0").unwrap().center();
    cx.update(|window, cx| {
        let result = window.dispatch_event(
            LongPressEvent {
                phase: TouchPhase::Started,
                start_position: position,
                position,
            }
            .to_platform_input(),
            cx,
        );
        assert!(result.default_prevented);
        let view = view.read(cx);
        assert_eq!(view.menu.page, Some(Page::Pane));
        assert_eq!(view.pane_menu_target(), Some("p0"));
        assert!(!view.menu.opening_right_click);
    });
    cx.update(|window, cx| {
        view.update(cx, |view, cx| view.dismiss_menu(window, cx));
        full_draw(window, cx).clear(cx);
    });
    cx.simulate_mouse_down(position, MouseButton::Right, Modifiers::default());
    view.read_with(cx, |view, _| {
        assert_eq!(view.menu.page, Some(Page::Pane));
        assert_eq!(view.pane_menu_target(), Some("p0"));
        assert!(view.menu.opening_right_click);
    });
}

#[gpui::test]
fn desktop_rail_agents_keep_no_pane_menu(cx: &mut TestAppContext) {
    let _desktop = crate::ipad_ui::desktop();
    let (view, cx) = rail_window(cx, 1);
    view.update(cx, |view, cx| {
        let mut listing = snapshot(1);
        listing.agents.truncate(1);
        listing.panes = vec![
            serde_json::from_value(serde_json::json!({
                "pane_id": "p0", "workspace_id": "w0", "tab_id": "t0", "label": "review",
                "cwd": "/repo", "foreground_cwd": "/repo", "focused": false,
                "right_click_passthrough": false,
            }))
            .unwrap(),
        ];
        view.live.snapshot = Some(Arc::new(listing));
        view.live.status = ConnectionStatus::Connected;
        cx.notify();
    });
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
    let position = cx.debug_bounds("rail-agent-local-p0").unwrap().center();
    cx.update(|window, cx| {
        window.dispatch_event(
            LongPressEvent {
                phase: TouchPhase::Started,
                start_position: position,
                position,
            }
            .to_platform_input(),
            cx,
        );
    });
    view.read_with(cx, |view, _| assert_ne!(view.menu.page, Some(Page::Pane)));
    cx.simulate_mouse_down(position, MouseButton::Right, Modifiers::default());
    view.read_with(cx, |view, _| assert_ne!(view.menu.page, Some(Page::Pane)));
}
