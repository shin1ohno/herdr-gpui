use super::*;
use crate::settings_window::{self, SheetFixture};
use core::prelude::v1::test;
use gpui::{prelude::*, *};

fn open(
    cx: &mut TestAppContext,
    width: f32,
) -> (Entity<SheprdWindow>, SheetFixture, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(|window, cx| {
        crate::bind_keys(cx);
        fixture_window(window, cx)
    });
    cx.simulate_resize(size(px(width), px(834.)));
    let fixture = cx.update(|_, cx| SheetFixture::prepare(view.downgrade(), false, cx));
    cx.update(|window, cx| view.update(cx, |view, cx| view.open_preferences(window, cx)));
    cx.run_until_parked();
    draw(cx);
    (view, fixture, cx)
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

fn visible(cx: &mut VisualTestContext, bounds: Bounds<Pixels>, selector: &'static str) {
    let rect = cx.debug_bounds(selector).unwrap();
    assert!(
        rect.size.width > px(0.) && rect.size.height > px(0.),
        "{selector}: {rect:?}"
    );
    assert!(
        rect.left() >= bounds.left() && rect.right() <= bounds.right(),
        "{selector}: {rect:?} outside {bounds:?}"
    );
    assert!(
        rect.top() >= bounds.top() && rect.bottom() <= bounds.bottom(),
        "{selector}: {rect:?} outside {bounds:?}"
    );
}

#[gpui::test]
fn geometry_and_keyboard_viewport_keep_header_body_and_done_visible(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, fixture, cx) = open(cx, 1194.);
    assert_eq!(
        cx.debug_bounds("menu-panel").unwrap().size,
        size(px(1000.), px(760.))
    );
    for width in [1194., 320.] {
        cx.simulate_resize(size(px(width), px(834.)));
        let mut reduced = None;
        for height in [420., 834.] {
            let bounds = Bounds::new(point(px(0.), px(0.)), size(px(width), px(height)));
            cx.simulate_window_visual_viewport_change(cx.window_handle(), bounds);
            draw(cx);
            for selector in [
                "menu-panel",
                "ipad-settings-sheet-header",
                "ipad-settings-sheet-body",
                "ipad-settings-sheet-done",
            ] {
                visible(cx, bounds, selector);
            }
            let panel = cx.debug_bounds("menu-panel").unwrap();
            assert_eq!(
                panel.size.width,
                px(if width == 320. { 288. } else { 1000. })
            );
            assert_eq!(
                cx.debug_bounds("ipad-settings-sheet-header")
                    .unwrap()
                    .size
                    .height,
                px(60.)
            );
            assert!(cx.debug_bounds("settings-footer-save").is_none());
            assert!(cx.debug_bounds("settings-footer-status").is_none());
            let body = cx
                .debug_bounds("ipad-settings-sheet-body")
                .unwrap()
                .size
                .height;
            if let Some(before) = reduced {
                assert!(body > before);
            } else {
                reduced = Some(body);
            }
        }
    }
    cx.update(|window, cx| {
        assert!(fixture.focused(window, cx));
        assert_eq!(cx.windows().len(), 1);
    });
    view.read_with(cx, |view, _| {
        assert_eq!(view.menu.page, Some(crate::menu::Page::Settings))
    });
}

#[gpui::test]
fn compact_navigation_drills_into_a_page_and_back(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (_, _, cx) = open(cx, 320.);
    assert!(cx.debug_bounds("settings-body").is_none());
    let general = cx
        .debug_bounds("settings-section-general")
        .unwrap()
        .center();
    cx.simulate_click(general, Modifiers::default());
    draw(cx);
    assert!(cx.debug_bounds("settings-body").is_some());
    assert!(cx.debug_bounds("settings-sections").is_none());
    let back = cx
        .debug_bounds("ipad-settings-sheet-back")
        .unwrap()
        .center();
    cx.simulate_click(back, Modifiers::default());
    draw(cx);
    assert!(cx.debug_bounds("settings-sections").is_some());
    assert!(cx.debug_bounds("settings-body").is_none());
}

fn dismiss(cx: &mut TestAppContext, action: &str) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, fixture, cx) = open(cx, 1194.);
    cx.update(|_, cx| fixture.dirty(cx));
    match action {
        "escape" => cx.simulate_keystrokes("escape"),
        "ctrl-w" => cx.simulate_keystrokes("ctrl-w"),
        "done" => {
            let done = cx
                .debug_bounds("ipad-settings-sheet-done")
                .unwrap()
                .center();
            cx.simulate_click(done, Modifiers::default());
        }
        "backdrop" => cx.simulate_click(point(px(4.), px(4.)), Modifiers::default()),
        _ => unreachable!(),
    }
    cx.run_until_parked();
    assert_eq!(*fixture.writes.lock().unwrap(), ["Nord"]);
    view.read_with(cx, |view, _| assert!(view.menu.page.is_none()));
    cx.update(|window, cx| {
        assert!(view.read(cx).focus.is_focused(window));
        assert_eq!(cx.windows().len(), 1);
    });
}

#[gpui::test]
fn escape_flushes_drafts_before_returning_focus(cx: &mut TestAppContext) {
    dismiss(cx, "escape");
}
#[gpui::test]
fn backdrop_flushes_drafts_before_returning_focus(cx: &mut TestAppContext) {
    dismiss(cx, "backdrop");
}
#[gpui::test]
fn done_flushes_drafts_before_returning_focus(cx: &mut TestAppContext) {
    dismiss(cx, "done");
}
#[gpui::test]
fn control_w_flushes_drafts_without_closing_main(cx: &mut TestAppContext) {
    dismiss(cx, "ctrl-w");
}

#[gpui::test]
fn page_search_keeps_typing_and_control_w_local(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (owner, fixture, cx) = open(cx, 1194.);
    cx.update(|window, cx| {
        fixture.model.update(cx, |view, cx| {
            view.select_section(settings_window::Section::Theme, window, cx)
        });
    });
    draw(cx);
    let values = cx.debug_bounds("settings-theme-search").unwrap();
    cx.simulate_click(
        point(values.center().x, values.top() + px(8.)),
        Modifiers::default(),
    );
    draw(cx);
    cx.update(|window, cx| {
        assert!(!fixture.focused(window, cx));
        assert!(!owner.read(cx).focus.is_focused(window));
    });
    cx.simulate_keystrokes("n o r d");
    cx.dispatch_action(crate::actions::SelectAll);
    cx.dispatch_action(crate::actions::Copy);
    assert_eq!(
        cx.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
            .as_deref(),
        Some("nord")
    );
    cx.update(|_, cx| fixture.dirty(cx));
    cx.simulate_keystrokes("ctrl-w");
    cx.run_until_parked();
    assert_eq!(*fixture.writes.lock().unwrap(), ["Nord"]);
    owner.read_with(cx, |view, _| assert!(view.menu.page.is_none()));
    cx.update(|window, cx| {
        assert!(owner.read(cx).focus.is_focused(window));
        assert_eq!(cx.windows().len(), 1);
    });
}

#[gpui::test]
fn interrupted_sheet_motion_reverses_from_its_current_position(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (owner, cx) = cx.add_window_view(fixture_window);
    let start = crate::motion::now();
    let midway = start + crate::motion::ENTER / 3;
    owner.update(cx, |view, _| {
        view.menu.appear.live = true;
        view.menu.page = Some(crate::menu::Page::Settings);
        view.tick_menu_appear_at(start);
        let before = view.menu.appear.backdrop(midway);
        assert!(before > 0. && before < 1.);
        view.menu.page = None;
        view.tick_menu_appear_at(midway);
        let (page, leaving) = view.menu.appear.leaving(midway).unwrap();
        assert_eq!(page, crate::menu::Page::Settings);
        assert!((leaving - before).abs() < 0.001);
        view.menu.page = Some(crate::menu::Page::Settings);
        view.tick_menu_appear_at(midway);
        assert!((view.menu.appear.backdrop(midway) - leaving).abs() < 0.001);
        assert!(view.menu.appear.leaving(midway).is_none());
        let settled = midway + crate::motion::ENTER;
        assert_eq!(view.menu.appear.backdrop(settled), 1.);
        view.menu.page = None;
        view.tick_menu_appear_at(settled);
        assert!(
            view.menu
                .appear
                .leaving(settled + crate::motion::LEAVE)
                .is_none()
        );
    });
}

#[gpui::test]
fn f6_and_close_actions_stay_in_the_sheet(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, fixture, cx) = open(cx, 1194.);
    cx.simulate_keystrokes("f6");
    cx.update(|window, cx| assert!(fixture.focused(window, cx)));
    cx.dispatch_action(crate::RunCommand {
        command: Command::CloseWindow,
    });
    cx.run_until_parked();
    view.read_with(cx, |view, _| assert!(view.menu.page.is_none()));
    assert_eq!(cx.windows().len(), 1);
}

#[gpui::test]
fn remote_entry_uses_the_existing_main_window(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, _, cx) = open(cx, 1194.);
    cx.update(|_, cx| settings_window::open_remote(view.downgrade(), cx));
    cx.run_until_parked();
    assert_eq!(cx.windows().len(), 1);
    view.read_with(cx, |view, _| {
        assert_eq!(view.menu.page, Some(crate::menu::Page::Settings))
    });
}

#[gpui::test]
fn prefix_chord_flushes_before_the_sheet_dismisses(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, fixture, cx) = open(cx, 1194.);
    view.update(cx, |view, _| {
        view.config.shortcut_style = crate::config::ShortcutStyle::HerdrPrefix
    });
    cx.update(|_, cx| fixture.dirty(cx));
    cx.simulate_keystrokes("ctrl-b c");
    cx.run_until_parked();
    assert_eq!(*fixture.writes.lock().unwrap(), ["Nord"]);
    view.read_with(cx, |view, _| {
        assert!(view.menu.page.is_none());
        assert!(!view.prefix_armed);
    });
    cx.update(|window, cx| assert!(view.read(cx).focus.is_focused(window)));
}
