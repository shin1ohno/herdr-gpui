use super::*;
use crate::sidebar::layout_tests::fixture_window;
use gpui::{EntityInputHandler, TestAppContext};

#[test]
fn control_is_one_shot_and_alt_can_be_combined_or_toggled_off() {
    let latch = Latch::default();
    latch.toggle_control();
    assert_eq!(latch.take("c"), Some(Keystroke::parse("ctrl-c").unwrap()));
    assert!(!latch.control());
    assert_eq!(latch.take("c"), None);
    latch.toggle_alt();
    latch.toggle_control();
    assert_eq!(
        latch.take("x"),
        Some(Keystroke::parse("ctrl-alt-x").unwrap())
    );
    assert!(!latch.alt());
    assert!(!latch.control());
    latch.toggle_alt();
    latch.toggle_alt();
    assert_eq!(latch.take("a"), None);
}

#[test]
fn multi_character_commits_stay_whole_and_do_not_consume_the_latch() {
    let latch = Latch::default();
    latch.toggle_alt();
    for text in ["", "abc", "日本", "e\u{301}", "\n"] {
        assert_eq!(latch.take(text), None);
        assert!(latch.alt());
    }
    assert_eq!(
        latch.take(" "),
        Some(Keystroke::parse("alt-space").unwrap())
    );
    assert!(!latch.alt());
}

#[gpui::test]
fn deferred_latched_commit_reaches_prefix_without_counting_as_hardware(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update_in(cx, |view, window, cx| {
        view.ipad.latch.toggle_control();
        view.replace_text_in_range(None, "b", window, cx);
        assert!(!view.ipad.latch.control());
        assert!(!view.prefix_armed, "the lease has not returned yet");
    });
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        assert!(view.prefix_armed);
        assert!(!view.ipad.keyboard.synthetic);
    });
    cx.update(|window, cx| assert!(!view.read(cx).ipad.keyboard.hardware(window)));
}

#[gpui::test]
fn desktop_behavior_does_not_consume_an_ipad_latch(cx: &mut TestAppContext) {
    let _desktop = crate::ipad_ui::desktop();
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update_in(cx, |view, window, cx| {
        view.ipad.latch.toggle_control();
        assert!(!view.ipad_latched_text("c", window, cx));
        assert!(view.ipad.latch.control());
    });
}
