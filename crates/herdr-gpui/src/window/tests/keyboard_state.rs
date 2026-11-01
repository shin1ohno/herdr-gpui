use super::super::keyboard_state::KeyboardState;
use super::*;
use gpui::{
    Bounds, Entity, TestAppContext, VisualContext as _, VisualTestContext, point, px, size,
};

fn inset(cx: &mut VisualTestContext, bottom: f32) {
    cx.simulate_resize(size(px(900.), px(800.)));
    cx.simulate_window_visual_viewport_change(
        cx.window_handle(),
        Bounds::new(point(px(0.), px(0.)), size(px(900.), px(800. - bottom))),
    );
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn hardware(view: &Entity<HerdrWindow>, cx: &mut VisualTestContext) -> bool {
    cx.update(|window, cx| view.read(cx).ipad.keyboard.hardware(window))
}

#[gpui::test]
fn assistant_inset_and_docked_keyboard_have_distinct_visibility(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(fixture_window);
    for bottom in [0., 55., 120., 300.] {
        inset(cx, bottom);
        assert_eq!(
            cx.update(|window, _| KeyboardState::soft_keyboard_shown(window)),
            bottom > 120.
        );
        assert!(!hardware(&view, cx));
    }
    cx.simulate_keystrokes("escape");
    assert!(
        !hardware(&view, cx),
        "soft keyboard events are not evidence"
    );
    inset(cx, 55.);
    cx.simulate_keystrokes("escape");
    assert!(hardware(&view, cx));
    inset(cx, 300.);
    assert!(!hardware(&view, cx));
    inset(cx, 0.);
    assert!(
        hardware(&view, cx),
        "evidence survives hiding the software keyboard"
    );
}

#[gpui::test]
fn synthetic_keys_and_desktop_behavior_do_not_count_as_hardware(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(fixture_window);
    inset(cx, 0.);
    view.update(cx, |view, _| view.ipad.keyboard.synthetic = true);
    cx.simulate_keystrokes("escape");
    assert!(!hardware(&view, cx));
    view.update(cx, |view, _| view.ipad.keyboard.synthetic = false);
    {
        let _desktop = crate::ipad_ui::desktop();
        cx.simulate_keystrokes("escape");
    }
    assert!(!hardware(&view, cx));
    cx.simulate_keystrokes("ctrl-b");
    assert!(
        hardware(&view, cx),
        "a consumed prefix still supplies evidence"
    );
    assert!(view.read_with(cx, |view, _| view.prefix_armed));
}

#[gpui::test]
fn keyboard_evidence_belongs_to_the_window_that_received_the_key(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (first, first_cx) = cx.add_window_view(fixture_window);
    inset(first_cx, 0.);
    let first_handle = first_cx.window_handle();
    let (second, second_cx) = cx.add_window_view(fixture_window);
    inset(second_cx, 0.);
    second_cx.simulate_keystrokes("escape");
    assert!(hardware(&second, second_cx));
    second_cx.update(|_, cx| {
        first_handle
            .update(cx, |_, window, cx| {
                assert!(!first.read(cx).ipad.keyboard.hardware(window));
            })
            .unwrap();
    });
}
