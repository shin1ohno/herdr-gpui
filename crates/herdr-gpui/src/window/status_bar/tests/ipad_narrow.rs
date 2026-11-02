#![allow(clippy::unwrap_used)]
use super::*;
use crate::{
    config::ShortcutStyle,
    controls::Command,
    sidebar::layout_tests::{fixture_window, full_draw},
};
use core::prelude::v1::test;

const WIDTHS: &[f32] = &[1194., 834., 500., 375., 320.];

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| full_draw(window, cx).clear(cx));
}

fn fixture(window: &mut Window, cx: &mut Context<HerdrWindow>) -> HerdrWindow {
    let mut view = fixture_window(window, cx);
    view.endpoints[0].label = "a very long host name on the local development machine".into();
    view.live.status = ConnectionStatus::Connected;
    view
}

fn inside(inner: Bounds<Pixels>, outer: Bounds<Pixels>) -> bool {
    inner.left() >= outer.left()
        && inner.right() <= outer.right()
        && inner.top() >= outer.top()
        && inner.bottom() <= outer.bottom()
}

#[gpui::test]
fn ipad_narrow_status_host_keeps_its_floor(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(fixture);
    let floor = view.read_with(cx, |view, _| {
        let host = fl!(
            "ipad-chrome-status-host",
            host = view.endpoints[0].label.as_str(),
            generation = 1
        );
        px((host.chars().count() as f32 * view.config.ui.size * 0.62).min(96.))
    });
    for style in [ShortcutStyle::Ipad, ShortcutStyle::HerdrPrefix] {
        view.update(cx, |view, cx| {
            view.config.shortcut_style = style;
            cx.notify();
        });
        for &width in WIDTHS {
            view.update(cx, |view, cx| {
                view.ipad.keyboard = Default::default();
                cx.notify();
            });
            cx.simulate_resize(size(px(width), px(834.)));
            draw(cx);
            let bar = cx.debug_bounds("connection-status").unwrap();
            let host = cx.debug_bounds("ipad-status-host").unwrap();
            if bar.size.width >= px(240.) {
                assert!(
                    host.size.width >= floor,
                    "{style:?} window {width}: host {host:?} below floor {floor:?}; bar {bar:?}"
                );
                assert!(inside(host, bar), "{width}: host {host:?} outside {bar:?}");
            }
            cx.simulate_keystrokes("escape");
            draw(cx);
            let host = cx.debug_bounds("ipad-status-host").unwrap();
            if bar.size.width >= px(240.) {
                assert!(
                    host.size.width >= floor,
                    "{style:?} window {width}: hardware chip displaced host {host:?} below {floor:?}"
                );
            }
        }
    }
}

#[gpui::test]
fn ipad_narrow_status_legend_keeps_whole_leading_pairs(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(fixture);
    for style in [ShortcutStyle::Ipad, ShortcutStyle::HerdrPrefix] {
        view.update(cx, |view, cx| {
            view.config.shortcut_style = style;
            cx.notify();
        });
        let actions = match style {
            ShortcutStyle::Ipad => [
                Command::Tab,
                Command::SplitRight,
                Command::ToggleSidebar,
                Command::Palette,
            ],
            ShortcutStyle::HerdrPrefix => [
                Command::Tab,
                Command::SplitRight,
                Command::ClosePane,
                Command::Keybinds,
            ],
        };
        for &width in WIDTHS {
            view.update(cx, |view, cx| {
                view.ipad.keyboard = Default::default();
                cx.notify();
            });
            cx.simulate_resize(size(px(width), px(834.)));
            for hardware in [false, true] {
                if hardware {
                    cx.simulate_keystrokes("escape");
                }
                draw(cx);
                let bar = cx.debug_bounds("connection-status").unwrap();
                let legend = cx.debug_bounds("status-keybinds").unwrap();
                let keyboard = cx.debug_bounds("status-keyboard").unwrap();
                assert_eq!(keyboard.size, size(px(44.), px(44.)));
                assert!(inside(keyboard, bar), "{width}: keyboard outside {bar:?}");
                let mut hidden = false;
                let mut visible = 0;
                for command in actions {
                    let selector: &str = format!("ipad-legend-{command:?}").leak();
                    let pair = cx.debug_bounds(selector).unwrap();
                    if inside(pair, legend) && inside(pair, bar) {
                        assert!(
                            !hidden,
                            "{style:?} {width}: {selector} follows a hidden pair"
                        );
                        visible += 1;
                    } else {
                        assert!(
                            pair.top() >= bar.bottom(),
                            "{style:?} window {width}: {selector} {pair:?} clipped by legend {legend:?}, bar {bar:?}"
                        );
                        hidden = true;
                    }
                }
                if width == 1194. {
                    assert_eq!(visible, actions.len(), "{style:?}: wide legend lost pairs");
                    // The legend is as wide as its pairs: no trailing room
                    // pushes it away from the keyboard button.
                    let last: &str = format!("ipad-legend-{:?}", actions[3]).leak();
                    assert_eq!(cx.debug_bounds(last).unwrap().right(), legend.right());
                }
            }
        }
    }
}

fn measured_text(
    text: SharedString,
    view: &Entity<HerdrWindow>,
    cx: &mut VisualTestContext,
) -> Pixels {
    cx.update(|window, cx| {
        let font = &view.read(cx).config.ui;
        window
            .text_system()
            .shape_line(
                text.clone(),
                px(font.size),
                &[TextRun {
                    len: text.len(),
                    font: font.font(),
                    color: black(),
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                }],
                None,
            )
            .width
    })
}

#[gpui::test]
fn ipad_narrow_short_host_has_no_extra_width(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture(window, cx);
        view.endpoints[0].label = "mini".into();
        view
    });
    let measured = measured_text(
        fl!("ipad-chrome-status-host", host = "mini", generation = 1).into(),
        &view,
        cx,
    );
    cx.simulate_resize(size(px(1194.), px(834.)));
    draw(cx);
    let host = cx.debug_bounds("ipad-status-host").unwrap();
    assert!(
        (host.size.width - measured).abs() <= px(1.),
        "short host {host:?} differs from measured {measured:?}"
    );
}

#[gpui::test]
fn ipad_narrow_short_host_leaves_hardware_text_whole(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture(window, cx);
        view.endpoints[0].label = "mini".into();
        view
    });
    cx.simulate_resize(size(px(1194.), px(834.)));
    let natural = measured_text(
        fl!("ipad-chrome-status-hardware-keyboard").into(),
        &view,
        cx,
    ) + px(STATUS_GLYPH + 5.);
    for style in [ShortcutStyle::Ipad, ShortcutStyle::HerdrPrefix] {
        view.update(cx, |view, cx| {
            view.config.shortcut_style = style;
            cx.notify();
        });
        cx.simulate_keystrokes("escape");
        draw(cx);
        let hardware = cx.debug_bounds("ipad-status-hardware").unwrap();
        assert!(
            hardware.size.width >= natural,
            "{style:?}: hardware chip {hardware:?} is narrower than measured {natural:?}"
        );
    }
}

fn connection_message_keeps_room(cx: &mut TestAppContext, width: f32) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (_, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        view.live.status = ConnectionStatus::Connected;
        view.local_error = Some("failed operation".into());
        view
    });
    cx.simulate_resize(size(px(width), px(834.)));
    draw(cx);
    let message = cx.debug_bounds("connection-message").unwrap();
    assert!(
        message.size.width >= px(60.),
        "window {width}: connection message {message:?} is narrower than 60pt"
    );
}

#[gpui::test]
fn ipad_narrow_connection_message_keeps_room_at_834(cx: &mut TestAppContext) {
    connection_message_keeps_room(cx, 834.);
}

#[gpui::test]
fn ipad_narrow_connection_message_keeps_room_at_500(cx: &mut TestAppContext) {
    connection_message_keeps_room(cx, 500.);
}

fn armed_prefix_stays_inside(cx: &mut TestAppContext, width: f32) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (_, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture(window, cx);
        view.config.shortcut_style = ShortcutStyle::HerdrPrefix;
        view.prefix_armed = true;
        view
    });
    cx.simulate_resize(size(px(width), px(834.)));
    draw(cx);
    let bar = cx.debug_bounds("connection-status").unwrap();
    let prefix = cx.debug_bounds("prefix-armed").unwrap();
    assert!(
        inside(prefix, bar),
        "window {width}: prefix {prefix:?} lies outside bar {bar:?}"
    );
}

#[gpui::test]
fn ipad_narrow_armed_prefix_stays_inside_at_375(cx: &mut TestAppContext) {
    armed_prefix_stays_inside(cx, 375.);
}

#[gpui::test]
fn ipad_narrow_armed_prefix_stays_inside_at_320(cx: &mut TestAppContext) {
    armed_prefix_stays_inside(cx, 320.);
}
