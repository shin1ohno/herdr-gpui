use super::*;
use core::prelude::v1::test;
use gpui::*;

fn keyboard(cx: &mut VisualTestContext, inset: f32) {
    cx.simulate_resize(size(px(1194.), px(834.)));
    cx.simulate_window_visual_viewport_change(
        cx.window_handle(),
        Bounds::new(point(px(0.), px(0.)), size(px(1194.), px(834. - inset))),
    );
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

#[gpui::test]
fn docked_keyboard_reserves_the_band_directly_above_its_inset(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (_, cx) = cx.add_window_view(fixture_window);
    keyboard(cx, 300.);
    let band = cx.debug_bounds("ipad-key-band").unwrap();
    assert_eq!(band.size.height, px(76.));
    assert_eq!(band.bottom(), px(534.));
    assert!(cx.debug_bounds("terminal").unwrap().bottom() <= band.top());
    for key in [
        "ipad-key-Escape",
        "ipad-key-Control",
        "ipad-key-Alt",
        "ipad-key-Paste",
    ] {
        assert_eq!(cx.debug_bounds(key).unwrap().size, size(px(52.), px(44.)));
    }
    keyboard(cx, 55.);
    assert!(cx.debug_bounds("ipad-key-band").is_none());
}

#[gpui::test]
fn keyboard_insets_do_not_add_touch_chrome_without_the_layout_guard(cx: &mut TestAppContext) {
    let (_, cx) = cx.add_window_view(fixture_window);
    keyboard(cx, 300.);
    assert!(cx.debug_bounds("ipad-key-band").is_none());
}

#[gpui::test]
fn touching_keys_keeps_terminal_focus_and_dispatches_escape_to_the_prefix(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(fixture_window);
    keyboard(cx, 300.);
    let focus = view.read_with(cx, |view, _| view.focus.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    cx.simulate_keystrokes("ctrl-b");
    assert!(view.read_with(cx, |view, _| view.prefix_armed));
    let escape = cx.debug_bounds("ipad-key-Escape").unwrap().center();
    cx.simulate_click(escape, Modifiers::default());
    cx.run_until_parked();
    assert!(!view.read_with(cx, |view, _| view.prefix_armed));
    cx.update(|window, _| assert!(focus.is_focused(window)));
}
