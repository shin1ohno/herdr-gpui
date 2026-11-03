use super::*;
use crate::window::viewport::simulate_window_controls;
use gpui::{Edges, Modifiers, TestAppContext, VisualTestContext, px, size};

mod sizes;

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        window.refresh();
        window.draw(cx).clear(cx);
    });
}

#[gpui::test]
fn compact_bar_clears_window_controls_and_follows_a_change(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (_, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(320.), px(834.)));
    draw(cx);
    let bar = cx.debug_bounds("ipad-compact-top-bar").unwrap();
    let menu = cx.debug_bounds("ipad-compact-menu").unwrap();
    assert_eq!(menu.left(), bar.left() + px(4.));

    // Shaped like a windowed iPadOS 26.5 sample: the controls on the left,
    // the window's corner allowance on the right.
    let controls = simulate_window_controls(Edges {
        left: px(70.),
        right: px(10.),
        ..Default::default()
    });
    draw(cx);
    let bar = cx.debug_bounds("ipad-compact-top-bar").unwrap();
    let menu = cx.debug_bounds("ipad-compact-menu").unwrap();
    let tabs = cx.debug_bounds("ipad-compact-tabs").unwrap();
    let group = cx.debug_bounds("ipad-compact-group-menu").unwrap();
    assert_eq!(bar.left(), px(0.));
    assert_eq!(bar.size, size(px(320.), px(52.)));
    assert_eq!(menu.left(), bar.left() + px(74.));
    assert!(tabs.left() >= menu.right());
    assert_eq!(group.right(), bar.right() - px(14.));

    cx.simulate_click(menu.center(), Modifiers::default());
    cx.run_until_parked();
    draw(cx);
    let panel = cx.debug_bounds("ipad-compact-sidebar").unwrap();
    assert_eq!(panel.top(), bar.bottom());

    drop(controls);
    draw(cx);
    let bar = cx.debug_bounds("ipad-compact-top-bar").unwrap();
    let menu = cx.debug_bounds("ipad-compact-menu").unwrap();
    assert_eq!(menu.left(), bar.left() + px(4.));
}

#[gpui::test]
fn mirrored_compact_bar_pads_the_physical_side_with_the_controls(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    crate::i18n::show(&herdr_i18n::Pseudo::Bidi.locale());
    let _controls = simulate_window_controls(Edges {
        left: px(70.),
        ..Default::default()
    });
    let (_, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(320.), px(834.)));
    draw(cx);
    let bar = cx.debug_bounds("ipad-compact-top-bar").unwrap();
    let menu = cx.debug_bounds("ipad-compact-menu").unwrap();
    let group = cx.debug_bounds("ipad-compact-group-menu").unwrap();
    assert_eq!(group.left(), bar.left() + px(74.));
    assert_eq!(menu.right(), bar.right() - px(4.));
}
