use super::skill_fixture;
use gpui::{Bounds, VisualContext as _, point, px, size};

#[gpui::test]
#[allow(clippy::unwrap_used)]
fn settings_controls_remain_above_the_software_keyboard(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(skill_fixture);
    view.update(cx, |view, cx| {
        view.accept_layout_choice(crate::config::LayoutMode::Orca, cx);
    });
    cx.simulate_resize(size(px(900.), px(800.)));
    let visible = Bounds::new(point(px(0.), px(24.)), size(px(900.), px(476.)));
    cx.simulate_window_visual_viewport_change(cx.window_handle(), visible);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    for selector in [
        "settings-body",
        "settings-footer-save",
        "settings-footer-reload",
    ] {
        let bounds = cx.debug_bounds(selector).unwrap();
        assert!(bounds.top() >= visible.top(), "{selector}: {bounds:?}");
        assert!(
            bounds.bottom() <= visible.bottom(),
            "{selector}: {bounds:?}"
        );
    }
}
