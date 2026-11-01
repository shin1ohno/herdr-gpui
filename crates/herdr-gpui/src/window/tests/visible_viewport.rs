use super::*;
use gpui::{Bounds, VisualContext as _, point, px, size};

#[gpui::test]
fn platform_occlusion_moves_controls_and_terminal_geometry_into_the_visible_area(
    cx: &mut gpui::TestAppContext,
) {
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(900.), px(800.)));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let original_rows = view.read_with(cx, |view, _| view.options.surface_size.rows);
    let visible = Bounds::new(point(px(0.), px(24.)), size(px(900.), px(476.)));
    cx.simulate_window_visual_viewport_change(cx.window_handle(), visible);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    for selector in ["sidebar", "device-footer", "device-settings", "terminal"] {
        let bounds = cx.debug_bounds(selector).unwrap();
        assert!(bounds.top() >= visible.top(), "{selector}: {bounds:?}");
        assert!(
            bounds.bottom() <= visible.bottom(),
            "{selector}: {bounds:?}"
        );
    }
    view.read_with(cx, |view, _| {
        assert!(view.options.surface_size.rows < original_rows);
        assert!(view.bounds.top() >= visible.top());
        assert!(view.bounds.bottom() <= visible.bottom());
        assert_eq!(
            view.options.surface_size,
            crate::terminal::viewport(
                f32::from(view.bounds.size.width),
                f32::from(view.bounds.size.height),
                view.cell_width,
                view.config.terminal.line_height(),
            )
        );
    });
    let picker = cx.debug_bounds("device-picker").unwrap();
    cx.simulate_click(picker.center(), gpui::Modifiers::default());
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let menu = cx.debug_bounds("menu-panel").unwrap();
    assert!(menu.top() >= visible.top() && menu.bottom() <= visible.bottom());
    view.update_in(cx, |view, window, cx| {
        view.dismiss_menu(window, cx);
        view.open_keybinds(window, cx);
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let dialog = cx.debug_bounds("menu-panel").unwrap();
    assert!(dialog.top() >= visible.top() && dialog.bottom() <= visible.bottom());
    cx.simulate_window_visual_viewport_change(
        cx.window_handle(),
        Bounds::new(point(px(0.), px(0.)), size(px(900.), px(800.))),
    );
    cx.update(|window, cx| window.draw(cx).clear(cx));
    view.read_with(cx, |view, _| {
        assert_eq!(view.options.surface_size.rows, original_rows)
    });
}
