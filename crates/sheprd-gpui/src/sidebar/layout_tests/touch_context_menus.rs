use super::*;
use crate::{menu::Page, state::ConnectionStatus};
use gpui::{InputEvent as _, LongPressEvent, TouchPhase};

const HOST: &str = "ssh:0123456789abcdef0123456789abcdef";

fn remote_window(window: &mut Window, cx: &mut Context<SheprdWindow>, rail: bool) -> SheprdWindow {
    let mut view = fixture_window(window, cx);
    // Saved-host menus edit the default catalog. The base fixture's explicit
    // socket intentionally disables that UI, so use a normal local sentinel.
    view.endpoints[0].connection.target = ConnectTarget::Local;
    view.sidebar_visible = !rail;
    view.sidebar_start_pending = false;
    view.endpoints[0].collapsed = true;
    let mut listing = snapshot(1);
    listing.workspaces[0].label = "remote workspace".into();
    listing.agents.truncate(1);
    listing.panes = vec![
        serde_json::from_value(serde_json::json!({
            "pane_id": "p0", "workspace_id": "w0", "tab_id": "t0", "label": "review",
            "cwd": "/repo", "foreground_cwd": "/repo", "focused": false,
            "right_click_passthrough": false,
        }))
        .unwrap(),
    ];
    let mut remote = crate::endpoint::Endpoint::new(
        HOST.into(),
        "Remote".into(),
        ConnectTarget::Ssh {
            target: "unused".into(),
            session: "default".into(),
        },
        true,
    );
    remote.live.snapshot = Some(Arc::new(listing));
    remote.live.status = ConnectionStatus::Connected;
    view.endpoints.push(remote);
    view
}

fn press(cx: &mut gpui::VisualTestContext, position: gpui::Point<Pixels>) {
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
        assert!(
            result.default_prevented,
            "the context menu claims the touch"
        );
    });
}

#[gpui::test]
fn host_long_press_opens_the_host_menu_in_sidebar_and_tooltipped_rail(
    cx: &mut gpui::TestAppContext,
) {
    for rail in [false, true] {
        let (view, cx) = cx.add_window_view(|window, cx| remote_window(window, cx, rail));
        cx.simulate_resize(size(px(800.), px(600.)));
        cx.update(|window, cx| full_draw(window, cx).clear(cx));
        let selector = format!("{}host-{HOST}", if rail { "rail-" } else { "" });
        let position = cx.debug_bounds(selector.leak()).unwrap().center();
        press(cx, position);
        view.read_with(cx, |view, _| {
            assert_eq!(view.menu.page, Some(Page::Host));
            assert!(view.menu.host.is_some());
            assert_eq!(view.menu.anchor, position);
            assert!(!view.menu.opening_right_click);
        });
    }
}

#[gpui::test]
fn workspace_long_press_opens_the_workspace_menu_in_sidebar_and_tooltipped_rail(
    cx: &mut gpui::TestAppContext,
) {
    for rail in [false, true] {
        let (view, cx) = cx.add_window_view(|window, cx| remote_window(window, cx, rail));
        cx.simulate_resize(size(px(800.), px(600.)));
        cx.update(|window, cx| full_draw(window, cx).clear(cx));
        let selector = if rail {
            format!("rail-workspace-{HOST}-w0")
        } else {
            format!("workspace-{HOST}-w0")
        };
        let position = cx.debug_bounds(selector.leak()).unwrap().center();
        press(cx, position);
        view.read_with(cx, |view, _| {
            assert_eq!(view.selected_endpoint, 1);
            assert_eq!(view.menu.page, Some(Page::Workspace));
            assert_eq!(crate::menu::workspace_tests::target_id(view), Some("w0"));
            assert_eq!(view.menu.anchor, position);
            assert!(!view.menu.opening_right_click);
        });
    }
}

#[gpui::test]
fn agent_long_press_opens_its_pane_menu_in_sidebar_and_rail(cx: &mut gpui::TestAppContext) {
    for rail in [false, true] {
        let (view, cx) = cx.add_window_view(|window, cx| remote_window(window, cx, rail));
        cx.simulate_resize(size(px(800.), px(600.)));
        cx.update(|window, cx| full_draw(window, cx).clear(cx));
        let selector = if rail {
            format!("rail-agent-{HOST}-p0")
        } else {
            format!("agent-{HOST}-p0")
        };
        let position = cx.debug_bounds(selector.leak()).unwrap().center();
        press(cx, position);
        view.read_with(cx, |view, _| {
            assert_eq!(view.selected_endpoint, 1);
            assert_eq!(view.menu.page, Some(Page::Pane));
            assert_eq!(view.pane_menu_target(), Some("p0"));
            assert_eq!(view.menu.anchor, position);
            assert!(!view.menu.opening_right_click);
        });
    }
}
