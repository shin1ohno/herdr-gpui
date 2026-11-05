use super::skill_fixture;
use crate::window::viewport::simulate_window_controls;
use gpui::{Edges, Modifiers, TestAppContext, px, size};

#[gpui::test]
fn settings_search_clears_native_controls_and_still_takes_focus(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(skill_fixture);
    cx.simulate_resize(size(px(900.), px(800.)));
    let mut baseline = None;
    for inset in [0., 66., 96., 0.] {
        let _controls = simulate_window_controls(Edges {
            left: px(inset),
            ..Default::default()
        });
        cx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
        let search = cx.debug_bounds("theme-search").unwrap();
        assert!(search.left() >= px(inset));
        assert!(search.size.width > px(0.));
        let body = cx.debug_bounds("settings-body").unwrap();
        let geometry = (body.top(), body.size.height, search.top());
        if let Some(expected) = baseline {
            assert_eq!(geometry, expected);
        } else {
            baseline = Some(geometry);
        }
        cx.simulate_click(search.center(), Modifiers::default());
        assert!(
            cx.update(|window, cx| view.read(cx).nav_search.read(cx).focus.is_focused(window)),
            "the actual search input receives the tap"
        );
    }
}
