#![allow(clippy::unwrap_used)]
use super::*;
use crate::{
    config::ShortcutStyle,
    controls::Command,
    sidebar::layout_tests::{fixture_window, full_draw},
};
use core::prelude::v1::test;

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
}

struct KeyboardButton(Entity<HerdrWindow>);

impl Render for KeyboardButton {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().children(self.0.update(cx, |view, cx| view.ipad_keyboard_button(cx)))
    }
}

#[gpui::test]
fn ipad_status_geometry_and_hardware_evidence(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(1194.), px(834.)));
    draw(cx);
    assert_eq!(
        cx.debug_bounds("connection-status").unwrap().size.height,
        px(44.)
    );
    assert_eq!(
        cx.debug_bounds("status-keyboard").unwrap().size,
        size(px(44.), px(44.))
    );
    assert!(cx.debug_bounds("ipad-status-host").is_some());
    assert!(cx.debug_bounds("ipad-status-hardware").is_none());
    cx.simulate_keystrokes("escape");
    draw(cx);
    assert!(cx.debug_bounds("ipad-status-hardware").is_some());
    cx.simulate_window_visual_viewport_change(
        cx.window_handle(),
        Bounds::new(point(px(0.), px(0.)), size(px(1194.), px(534.))),
    );
    draw(cx);
    assert!(cx.debug_bounds("ipad-status-hardware").is_none());
    view.read_with(cx, |view, _| assert_eq!(view.status_bar_height(), 44.));
}

#[gpui::test]
fn ipad_status_excludes_desktop_items_but_keeps_prefix_and_error(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        view.prefix_armed = true;
        view.local_error = Some("failed operation".into());
        view.config.show_system_load = true;
        view.config.show_gpu_load = true;
        view
    });
    draw(cx);
    assert!(cx.debug_bounds("prefix-armed").is_some());
    assert!(cx.debug_bounds("connection-message").is_some());
    for selector in [
        "status-theme",
        "status-version",
        "report-issue",
        "status-caffeine",
        "agent-model",
        "system-load",
        "gpu-load",
        "network-quality",
        "listening-ports",
    ] {
        assert!(cx.debug_bounds(selector).is_none(), "{selector}");
    }
    let legend = cx.debug_bounds("status-keybinds").unwrap();
    cx.simulate_click(legend.center(), Modifiers::default());
    view.read_with(cx, |view, _| {
        assert_eq!(view.menu.page, Some(crate::menu::Page::Keybinds))
    });
}

#[gpui::test]
fn ipad_legend_uses_the_selected_style_and_keyboard_stays_reachable(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(fixture_window);
    for style in [ShortcutStyle::Ipad, ShortcutStyle::HerdrPrefix] {
        view.update(cx, |view, cx| {
            view.config.shortcut_style = style;
            let command = if style == ShortcutStyle::Ipad {
                Command::Palette
            } else {
                Command::Keybinds
            };
            assert!(view.shortcut_hint(command).is_some());
            view.endpoints[0].label =
                "a very long host label that must yield to the keyboard button".into();
            cx.notify();
        });
        for width in [1194., 500., 320.] {
            cx.simulate_resize(size(px(width), px(834.)));
            draw(cx);
            let bar = cx.debug_bounds("connection-status").unwrap();
            let keyboard = cx.debug_bounds("status-keyboard").unwrap();
            assert!(
                keyboard.left() >= bar.left() && keyboard.right() <= bar.right(),
                "{width}: {keyboard:?} inside {bar:?}"
            );
            assert!(cx.debug_bounds("status-keybinds").is_some());
            assert_eq!(
                cx.debug_bounds("ipad-legend-ToggleSidebar").is_some(),
                style == ShortcutStyle::Ipad
            );
            assert_eq!(
                cx.debug_bounds("ipad-legend-ClosePane").is_some(),
                style == ShortcutStyle::HerdrPrefix
            );
        }
    }
}

#[gpui::test]
fn keyboard_button_is_a_behavior_gate_not_a_layout_gate(cx: &mut TestAppContext) {
    let (_, cx) =
        cx.add_window_view(|window, cx| KeyboardButton(cx.new(|cx| fixture_window(window, cx))));
    draw(cx);
    assert!(cx.debug_bounds("status-keyboard").is_some());
    let _desktop = crate::ipad_ui::desktop();
    draw(cx);
    assert!(cx.debug_bounds("status-keyboard").is_none());
}

#[gpui::test]
fn keyboard_behavior_does_not_change_desktop_chrome_geometry(cx: &mut TestAppContext) {
    let (_, cx) = cx.add_window_view(fixture_window);
    draw(cx);
    let selectors = [
        "connection-status",
        "status-theme",
        "status-keybinds",
        "status-version",
    ];
    let enabled = selectors.map(|selector| cx.debug_bounds(selector).unwrap());
    assert!(cx.debug_bounds("status-keyboard").is_none());
    let _desktop = crate::ipad_ui::desktop();
    draw(cx);
    let disabled = selectors.map(|selector| cx.debug_bounds(selector).unwrap());
    assert_eq!(enabled, disabled);
}
