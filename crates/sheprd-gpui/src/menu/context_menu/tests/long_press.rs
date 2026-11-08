use super::*;
use std::time::Duration;

#[gpui::test]
fn touch_context_menu_wins_over_tooltip_and_does_not_click(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, _| Fixture::default());
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let position = point(px(30.), px(30.));
    touch(cx, TouchPhase::Started, position);
    cx.executor()
        .advance_clock(GestureTuning::default().long_press_duration + Duration::from_millis(1));
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert_eq!(view.opened, [(position, false)]);
        assert_eq!(view.tooltip_builds.get(), 0);
    });
    touch(cx, TouchPhase::Ended, position);
    view.read_with(cx, |view, _| assert_eq!(view.clicks, 0));
}

#[gpui::test]
fn context_menu_keeps_right_click_and_ignores_other_long_press_phases(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, _| Fixture::default());
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let position = point(px(30.), px(30.));
    cx.simulate_mouse_down(position, MouseButton::Right, Modifiers::default());
    view.read_with(cx, |view, _| assert_eq!(view.opened, [(position, true)]));
    for phase in [TouchPhase::Moved, TouchPhase::Ended, TouchPhase::Cancelled] {
        assert!(!long_press(cx, phase, position));
    }
    assert!(long_press(cx, TouchPhase::Started, position));
    view.read_with(cx, |view, _| {
        assert_eq!(view.opened, [(position, true), (position, false)]);
    });
}

#[gpui::test]
fn context_menu_respects_scroll_clipping_and_occlusion(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, _| Fixture::default());
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(!long_press(
        cx,
        TouchPhase::Started,
        point(px(30.), px(150.))
    ));
    view.update(cx, |view, cx| {
        view.covered = true;
        cx.notify();
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(!long_press(
        cx,
        TouchPhase::Started,
        point(px(30.), px(30.))
    ));
    view.read_with(cx, |view, _| assert!(view.opened.is_empty()));
}
