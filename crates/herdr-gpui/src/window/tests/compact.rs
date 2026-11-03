use super::*;
use core::prelude::v1::test;
use gpui::*;

#[gpui::test]
fn compact_width_hides_the_column_and_strip_and_keeps_scrolling_keys(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(320.), px(834.)));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    view.read_with(cx, |view, cx| {
        assert!(view.ipad_compact());
        assert_eq!(view.sidebar_column_width(320.), None);
        assert!(view.strip_hidden(view.group_slots()[0].id, cx));
    });
    assert!(cx.debug_bounds("sidebar").is_none());
    assert!(cx.debug_bounds("ipad-key-band").is_none());
    assert_eq!(
        cx.debug_bounds("ipad-compact-key-row").unwrap().size.height,
        px(56.)
    );
    cx.simulate_resize(size(px(560.), px(834.)));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(!view.read_with(cx, |view, _| view.ipad_compact()));
    assert!(cx.debug_bounds("ipad-compact-key-row").is_none());
}

#[gpui::test]
fn narrow_desktop_windows_keep_their_column_and_do_not_get_touch_keys(cx: &mut TestAppContext) {
    let _desktop = crate::ipad_ui::desktop();
    let (view, cx) = cx.add_window_view(fixture_window);
    cx.simulate_resize(size(px(320.), px(834.)));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(!view.read_with(cx, |view, _| view.ipad_compact()));
    assert!(cx.debug_bounds("ipad-compact-key-row").is_none());
}

fn next_frame(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.simulate_next_frame(cx));
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

#[gpui::test]
fn compact_font_applies_before_the_next_frame_and_restores_without_saving(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(fixture_window);
    let original = view.read_with(cx, |view, _| view.config.terminal.size);
    cx.simulate_resize(size(px(320.), px(834.)));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    next_frame(cx);
    assert_eq!(view.read_with(cx, |view, _| view.config.terminal.size), 13.);
    cx.simulate_resize(size(px(900.), px(834.)));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    next_frame(cx);
    assert_eq!(
        view.read_with(cx, |view, _| view.config.terminal.size),
        original
    );
    assert!(view.read_with(cx, |view, _| view.ipad.compact_font.is_none()));
    cx.simulate_resize(size(px(320.), px(834.)));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    next_frame(cx);
    view.update(cx, |view, cx| view.set_terminal_font_size(17., cx));
    cx.simulate_resize(size(px(900.), px(834.)));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    next_frame(cx);
    assert_eq!(view.read_with(cx, |view, _| view.config.terminal.size), 17.);
}

fn compact_fixture(window: &mut Window, cx: &mut Context<HerdrWindow>) -> HerdrWindow {
    let mut view = fixture_window(window, cx);
    let mut snapshot: herdr_client::protocol::ClientShellSnapshot = serde_json::from_str(
        include_str!("../../../../herdr-protocol/tests/fixtures/endpoint-snapshot-v1.json"),
    )
    .unwrap();
    let mut second = snapshot.panes[0].clone();
    second.pane_id = "w1:p2".into();
    second.focused = false;
    snapshot.panes.push(second);
    snapshot.tabs[0].label = "a shell with a deliberately very long name".into();
    view.live.snapshot = Some(Arc::new(snapshot));
    view
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn click(cx: &mut VisualTestContext, selector: &'static str) {
    let at = cx.debug_bounds(selector).unwrap().center();
    cx.simulate_click(at, Modifiers::default());
    cx.run_until_parked();
    draw(cx);
}

#[gpui::test]
fn compact_header_pager_and_keys_reserve_their_own_room(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (_, cx) = cx.add_window_view(compact_fixture);
    cx.simulate_resize(size(px(320.), px(834.)));
    draw(cx);
    let header = cx.debug_bounds("ipad-compact-top-bar").unwrap();
    let pager = cx.debug_bounds("ipad-pane-pager").unwrap();
    let keys = cx.debug_bounds("ipad-compact-key-row").unwrap();
    assert_eq!(header.size.height, px(52.));
    assert_eq!(pager.size.height, px(36.));
    assert_eq!(pager.top(), header.bottom());
    assert_eq!(keys.size.height, px(56.));
    assert!(cx.debug_bounds("terminal").unwrap().top() >= pager.bottom());
    assert!(cx.debug_bounds("terminal").unwrap().bottom() <= keys.top());
    assert!(cx.debug_bounds("sidebar").is_none());
    assert!(cx.debug_bounds("titlebar").is_none());
    for selector in [
        "ipad-compact-menu",
        "ipad-compact-tabs",
        "ipad-compact-group-menu",
    ] {
        let control = cx.debug_bounds(selector).unwrap();
        assert!(control.left() >= header.left() && control.right() <= header.right());
        assert!(control.size.height >= px(44.));
    }
}

#[gpui::test]
fn compact_overlay_opens_closes_and_leaves_saved_sidebar_state_alone(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(compact_fixture);
    cx.simulate_resize(size(px(320.), px(834.)));
    draw(cx);
    let saved = view.read_with(cx, |view, _| (view.sidebar_visible, view.sidebar_width));
    click(cx, "ipad-compact-menu");
    assert!(view.read_with(cx, |view, _| view.ipad.overlay_open.is_open()));
    let panel = cx.debug_bounds("ipad-compact-sidebar").unwrap();
    assert_eq!(panel.size.width, px(300.));
    assert_eq!(cx.debug_bounds("sidebar").unwrap().size.width, px(300.));
    assert_eq!(
        panel.top(),
        cx.debug_bounds("ipad-compact-top-bar").unwrap().bottom()
    );
    let scrim = cx.debug_bounds("ipad-compact-scrim").unwrap();
    let at = point(scrim.right() - px(5.), scrim.top() + px(10.));
    cx.simulate_click(at, Modifiers::default());
    cx.run_until_parked();
    draw(cx);
    assert!(!view.read_with(cx, |view, _| view.ipad.overlay_open.is_open()));
    assert!(cx.debug_bounds("ipad-compact-overlay").is_none());
    click(cx, "ipad-compact-menu");
    click(cx, "ipad-compact-menu");
    assert!(cx.debug_bounds("ipad-compact-overlay").is_none());
    assert_eq!(
        view.read_with(cx, |view, _| (view.sidebar_visible, view.sidebar_width)),
        saved
    );
    click(cx, "ipad-compact-menu");
    view.update(cx, |view, cx| {
        Arc::make_mut(view.live.snapshot.as_mut().unwrap()).focused_pane_id = Some("w1:p2".into());
        cx.notify();
    });
    draw(cx);
    assert!(
        cx.debug_bounds("ipad-compact-overlay").is_none(),
        "selection closes the drawer"
    );
}

#[gpui::test]
fn compact_header_opens_navigation_palette_and_group_menu(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(compact_fixture);
    cx.simulate_resize(size(px(320.), px(834.)));
    draw(cx);
    click(cx, "ipad-compact-menu");
    click(cx, "ipad-compact-tabs");
    view.read_with(cx, |view, _| {
        assert_eq!(view.menu.page, Some(crate::menu::Page::Palette));
        assert!(!view.ipad.overlay_open.is_open());
    });
    assert!(cx.debug_bounds("palette-agent-filter-all-agents").is_some());
    view.update_in(cx, |view, window, cx| view.dismiss_menu(window, cx));
    draw(cx);
    click(cx, "ipad-compact-group-menu");
    assert_eq!(
        view.read_with(cx, |view, _| view.menu.page),
        Some(crate::menu::Page::Group)
    );
}

#[gpui::test]
fn compact_chrome_mirrors_and_overlay_stays_above_the_keyboard(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    crate::i18n::show(&herdr_i18n::Pseudo::Bidi.locale());
    let (_, cx) = cx.add_window_view(compact_fixture);
    cx.simulate_resize(size(px(320.), px(834.)));
    let visible = Bounds::new(point(px(0.), px(24.)), size(px(320.), px(510.)));
    cx.simulate_window_visual_viewport_change(cx.window_handle(), visible);
    draw(cx);
    let header = cx.debug_bounds("ipad-compact-top-bar").unwrap();
    let menu = cx.debug_bounds("ipad-compact-menu").unwrap();
    let group = cx.debug_bounds("ipad-compact-group-menu").unwrap();
    assert!(menu.left() > group.right());
    assert_eq!(header.top(), visible.top());
    click(cx, "ipad-compact-menu");
    let panel = cx.debug_bounds("ipad-compact-sidebar").unwrap();
    assert_eq!(panel.right(), visible.right());
    assert_eq!(panel.top(), header.bottom());
    assert_eq!(panel.bottom(), visible.bottom());
}
