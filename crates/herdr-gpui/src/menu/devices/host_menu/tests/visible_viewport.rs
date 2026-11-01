use super::*;
use gpui::{Bounds, VisualContext as _};

#[gpui::test]
fn a_host_menu_stays_above_the_software_keyboard(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        add_host(&mut view);
        view
    });
    cx.simulate_resize(size(px(800.), px(800.)));
    let visible = Bounds::new(point(px(0.), px(24.)), size(px(800.), px(476.)));
    cx.simulate_window_visual_viewport_change(cx.window_handle(), visible);
    view.update_in(cx, |view, window, cx| {
        view.open_host_menu(HOST, point(px(780.), px(480.)), window, cx);
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let menu = cx.debug_bounds("menu-panel").unwrap();
    assert!(
        menu.top() >= visible.top() && menu.bottom() <= visible.bottom(),
        "{menu:?} outside {visible:?}"
    );
    assert!(
        menu.left() >= visible.left() && menu.right() <= visible.right(),
        "{menu:?} outside {visible:?}"
    );
}
