use super::*;
use gpui::VisualContext as _;

fn assert_visible(cx: &mut VisualTestContext, visible: Bounds<Pixels>, selector: &'static str) {
    let bounds = cx.debug_bounds(selector).unwrap();
    assert!(bounds.size.height > px(0.), "{selector}: {bounds:?}");
    assert!(
        bounds.top() >= visible.top() && bounds.bottom() <= visible.bottom(),
        "{selector}: {bounds:?} outside {visible:?}"
    );
    assert!(
        bounds.left() >= visible.left() && bounds.right() <= visible.right(),
        "{selector}: {bounds:?} outside {visible:?}"
    );
}

#[gpui::test]
fn add_device_keeps_actions_visible_as_the_keyboard_changes_the_viewport(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    for width in [1000., 360.] {
        cx.simulate_resize(size(px(width), px(800.)));
        view.update(cx, |view, cx| show_form(view, Step::Form, cx));
        let mut keyboard_body_height = None;
        for height in [476., 776.] {
            let visible = Bounds::new(point(px(0.), px(24.)), size(px(width), px(height)));
            cx.simulate_window_visual_viewport_change(cx.window_handle(), visible);
            cx.update(|window, cx| window.draw(cx).clear(cx));
            for selector in [
                "device-setup-dialog",
                "device-setup-header",
                "device-setup-body",
                "device-setup-footer",
                "device-setup-submit",
            ] {
                assert_visible(cx, visible, selector);
            }
            let body = cx.debug_bounds("device-setup-body").unwrap();
            let footer = cx.debug_bounds("device-setup-footer").unwrap();
            assert!(body.bottom() <= footer.top());
            if let Some(keyboard_body_height) = keyboard_body_height {
                assert!(body.size.height > keyboard_body_height);
            } else {
                keyboard_body_height = Some(body.size.height);
            }
        }
    }
}

#[gpui::test]
fn add_device_scrolls_recovery_advice_without_moving_the_actions(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(fixture_window);
    for width in [1000., 360.] {
        cx.simulate_resize(size(px(width), px(800.)));
        let visible = Bounds::new(point(px(0.), px(24.)), size(px(width), px(476.)));
        cx.simulate_window_visual_viewport_change(cx.window_handle(), visible);
        view.update(cx, |view, cx| {
            let request = setup::Request::new("dev@remote.test", "Remote", "").unwrap();
            show_form(view, Step::Confirm(request, Offer::Update), cx);
            view.set_device_error(Some("A bounded diagnostic line.\n".repeat(24)));
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        for selector in [
            "device-setup-header",
            "device-setup-body",
            "device-setup-footer",
            "device-setup-cancel",
        ] {
            assert_visible(cx, visible, selector);
        }
        let body = cx.debug_bounds("device-setup-body").unwrap();
        let footer = cx.debug_bounds("device-setup-footer").unwrap();
        let before = cx.debug_bounds("device-setup-status").unwrap();
        assert!(before.bottom() > body.bottom(), "{before:?} in {body:?}");
        cx.simulate_event(ScrollEvent {
            position: body.center(),
            delta: ScrollDelta::Pixels(point(px(0.), px(-10_000.))),
            ..Default::default()
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let after = cx.debug_bounds("device-setup-status").unwrap();
        assert!(after.top() < before.top());
        assert!(after.bottom() <= body.bottom(), "{after:?} in {body:?}");
        assert!(after.top() >= body.top(), "{after:?} in {body:?}");
        assert_eq!(cx.debug_bounds("device-setup-footer").unwrap(), footer);
        assert_visible(cx, visible, "device-setup-cancel");
    }
}
