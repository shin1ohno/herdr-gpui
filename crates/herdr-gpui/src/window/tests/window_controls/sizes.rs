use super::*;
use gpui::{Bounds, VisualContext, point};

#[gpui::test]
fn controls_keep_their_touch_targets_at_compact_widths_in_both_directions(
    test_cx: &mut TestAppContext,
) {
    let _ipad = crate::ipad_ui::ipad_layout();
    for locale in ["en-US", "en-XB"] {
        crate::i18n::show(&locale.parse().unwrap());
        let _controls = simulate_window_controls(Edges {
            left: px(66.),
            right: px(9.5),
            ..Default::default()
        });
        let (view, cx) = test_cx.add_window_view(fixture_window);
        for width in [320., 375., 559.] {
            cx.simulate_resize(size(px(width), px(834.)));
            draw(cx);
            let bar = cx.debug_bounds("ipad-compact-top-bar").unwrap();
            for selector in [
                "ipad-compact-menu",
                "ipad-compact-tabs",
                "ipad-compact-group-menu",
            ] {
                let control = cx.debug_bounds(selector).unwrap();
                assert!(
                    control.left() >= bar.left() + px(66.),
                    "{locale}: {selector}"
                );
                assert!(
                    control.right() <= bar.right() - px(9.5),
                    "{locale}: {selector}"
                );
                assert!(control.size.width > px(0.));
                assert!(control.size.height >= px(44.));
            }
            let menu = cx.debug_bounds("ipad-compact-menu").unwrap();
            let group = cx.debug_bounds("ipad-compact-group-menu").unwrap();
            assert_eq!(menu.left() > group.left(), locale == "en-XB");
            cx.simulate_click(menu.center(), Modifiers::default());
            cx.run_until_parked();
            draw(cx);
            assert!(view.read_with(cx, |view, _| view.ipad.overlay_open.is_open()));
            let panel = cx.debug_bounds("ipad-compact-sidebar").unwrap();
            assert_eq!(panel.top(), bar.bottom());
            assert_eq!(panel.size.width, px(300.));
            cx.simulate_click(menu.center(), Modifiers::default());
            cx.run_until_parked();
            draw(cx);
            assert!(!view.read_with(cx, |view, _| view.ipad.overlay_open.is_open()));
        }
    }
}

#[gpui::test]
fn control_clearance_is_added_to_safe_area_without_insetting_the_terminal(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let _controls = simulate_window_controls(Edges {
        left: px(66.),
        right: px(9.5),
        ..Default::default()
    });
    let (_, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(375.), px(834.)));
    let visible = Bounds::new(point(px(12.), px(32.)), size(px(351.), px(480.)));
    cx.simulate_window_visual_viewport_change(cx.window_handle(), visible);
    draw(cx);
    let bar = cx.debug_bounds("ipad-compact-top-bar").unwrap();
    let menu = cx.debug_bounds("ipad-compact-menu").unwrap();
    let group = cx.debug_bounds("ipad-compact-group-menu").unwrap();
    assert_eq!(bar.origin, visible.origin);
    assert_eq!(bar.size.height, px(52.));
    assert_eq!(menu.left(), visible.left() + px(70.));
    assert_eq!(group.right(), visible.right() - px(13.5));
    let terminal = cx.debug_bounds("terminal").unwrap();
    assert!(terminal.left() < menu.left());
    assert!(terminal.right() > group.right());
    assert!(terminal.bottom() <= visible.bottom());
}
