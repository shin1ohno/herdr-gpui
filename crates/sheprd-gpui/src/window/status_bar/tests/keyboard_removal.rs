#![allow(clippy::unwrap_used)]

use super::*;
use crate::sidebar::layout_tests::{fixture_window, full_draw};
use core::prelude::v1::test;

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
}

#[gpui::test]
fn ipad_status_ends_at_shortcuts_or_expand_hint_without_a_keyboard_slot(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(|window, cx| {
        let view = fixture_window(window, cx);
        window.focus(&view.focus, cx);
        view
    });
    for width in [560., 1194.] {
        cx.simulate_resize(size(px(width), px(834.)));
        for collapsed in [false, true] {
            view.update(cx, |view, cx| {
                view.sidebar_visible = !collapsed;
                cx.notify();
            });
            draw(cx);
            let bar = cx.debug_bounds("connection-status").unwrap();
            assert!(cx.debug_bounds("status-keyboard").is_none());
            let last = if collapsed {
                "ipad-rail-hint"
            } else {
                "status-keybinds"
            };
            let last = cx.debug_bounds(last).unwrap();
            assert_eq!(
                last.right(),
                bar.right() - px(12.),
                "no reserved keyboard slot"
            );
            assert_eq!(last.size.height, px(44.));
            let shortcuts = cx.debug_bounds("status-keybinds").unwrap();
            cx.simulate_click(shortcuts.center(), Modifiers::default());
            draw(cx);
            view.read_with(cx, |view, _| assert!(view.ipad.guide.manual));
            cx.simulate_keystrokes("escape");
            draw(cx);
            view.read_with(cx, |view, _| {
                assert!(!view.ipad.guide.manual);
                assert!(!view.prefix_armed);
            });
        }
    }
}

#[gpui::test]
fn terminal_tap_still_recovers_focus_without_a_status_keyboard_button(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(1194.), px(834.)));
    draw(cx);
    cx.update(|window, cx| {
        window.dismiss_virtual_keyboard();
        window.blur(cx);
    });
    let terminal = cx.debug_bounds("terminal").unwrap();
    cx.simulate_click(terminal.center(), Modifiers::default());
    view.update_in(cx, |view, window, _| {
        assert!(view.focus.is_focused(window));
        assert!(!view.ipad.guide.manual);
    });
    assert!(cx.debug_bounds("status-keyboard").is_none());
}
