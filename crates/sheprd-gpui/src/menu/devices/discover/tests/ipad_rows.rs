use super::*;
use gpui::{Modifiers, px, size};

#[gpui::test]
fn ipad_nearby_rows_are_touch_sized_and_show_only_the_highlight_check(
    cx: &mut gpui::TestAppContext,
) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = form::open(cx, form::report);
    cx.simulate_resize(size(px(1194.), px(834.)));
    form::settle(cx);
    let heading = cx.debug_bounds("devices-discover-heading-caps").unwrap();
    let status = cx.debug_bounds("device-discovery-status").unwrap();
    assert!(status.left() >= heading.right());
    for selector in ["device-suggestion-0", "device-suggestion-1"] {
        assert_eq!(cx.debug_bounds(selector).unwrap().size.height, px(56.));
    }
    assert!(
        cx.debug_bounds("device-discovery-again")
            .unwrap()
            .size
            .height
            >= px(44.)
    );
    assert!(cx.debug_bounds("device-suggestion-check-0").is_none());
    let row = cx.debug_bounds("device-suggestion-0").unwrap().center();
    cx.simulate_click(row, Modifiers::default());
    form::settle(cx);
    assert!(cx.debug_bounds("device-suggestion-check-0").is_some());
    assert!(cx.debug_bounds("device-suggestion-check-1").is_none());
    assert_eq!(
        form::fields(&view, cx),
        ("m4max.tail1.ts.net".into(), "m4max".into())
    );
    view.update(cx, |view, cx| {
        view.menu.device_setup.as_ref().unwrap().fields[0]
            .clone()
            .update(cx, |field, cx| {
                field.set_text_selected("unmatched.test", cx)
            });
    });
    form::settle(cx);
    assert!(cx.debug_bounds("device-suggestion-check-0").is_none());
}
