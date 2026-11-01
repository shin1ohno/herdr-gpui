use super::*;
use gpui::{Styled, px, rgb, size};

#[gpui::test]
fn ipad_sidebar_default_and_saved_width_share_the_existing_bounds(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(layout_tests::fixture_window);
    cx.simulate_resize(size(px(1194.), px(834.)));
    cx.update(|window, cx| layout_tests::full_draw(window, cx).clear(cx));
    assert_eq!(cx.debug_bounds("sidebar").unwrap().size.width, px(232.));
    let _ipad = crate::ipad_ui::ipad_layout();
    cx.update(|window, cx| layout_tests::full_draw(window, cx).clear(cx));
    assert_eq!(cx.debug_bounds("sidebar").unwrap().size.width, px(300.));
    view.update(cx, |view, cx| {
        view.sidebar_width = Some(350.);
        cx.notify();
    });
    cx.update(|window, cx| layout_tests::full_draw(window, cx).clear(cx));
    assert_eq!(cx.debug_bounds("sidebar").unwrap().size.width, px(350.));
    assert_eq!(super::super::sidebar_width(None, 320.), 80.);
    assert_eq!(super::super::sidebar_width(Some(350.), 320.), 80.);
    assert_eq!(super::super::sidebar_width(None, 200.), 0.);
}

#[test]
fn ipad_dot_theme_roles_preserve_filled_and_hollow_statuses() {
    use super::super::agents::status_dot;
    let theme = Theme::builtin("Nord").unwrap();
    let _ipad = crate::ipad_ui::ipad_layout();
    let indicators = Indicators::new(None, StatusIcons::Dots, false, false, &theme);
    for (status, color, filled) in [
        (AgentStatus::Idle, theme.palette[6], false),
        (AgentStatus::Done, theme.palette[2], true),
        (AgentStatus::Blocked, theme.palette[1], true),
        (AgentStatus::Unknown, theme.subtext(), true),
    ] {
        let mut dot = status_dot(status, 16., indicators);
        assert_eq!(dot.style().border_color, Some(rgb(color).into()));
        assert_eq!(dot.style().background, filled.then(|| rgb(color).into()));
        let diameter = if status == AgentStatus::Unknown {
            6.
        } else {
            16.
        };
        assert_eq!(dot.style().size.width, Some(px(diameter).into()));
    }
    let mut working = status_dot(AgentStatus::Working, 16., indicators);
    assert_eq!(working.style().size.width, Some(px(16.).into()));
    assert_eq!(indicators.color(AgentStatus::Working), theme.palette[3]);
    let _desktop = crate::ipad_ui::desktop();
    let desktop = Indicators::new(None, StatusIcons::Dots, false, false, &theme);
    assert_eq!(desktop.color(AgentStatus::Done), 0x94e2d5);
}
