use super::*;
use crate::config::ShortcutStyle;

fn ipad_window(window: &mut Window, cx: &mut Context<HerdrWindow>) -> HerdrWindow {
    let mut view = fixture_window(window, cx);
    view.config.shortcut_style = ShortcutStyle::Ipad;
    // The palette excludes its own command; a real GUI override supplies the
    // exact Apple label on Linux without changing the platform catalog.
    view.config.keybindings = crate::keymap::Keymap::with_overrides(
        &std::collections::BTreeMap::from([(
            "new_tab".into(),
            crate::keymap::Binding::One("cmd-shift-p".into()),
        )]),
        &Default::default(),
        &Default::default(),
    )
    .unwrap();
    view
}

fn open_tab(view: &Entity<HerdrWindow>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.open_palette(Filter::Commands, window, cx);
            view.filter_palette("cmd-shift-p", cx);
        })
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

#[gpui::test]
fn shortcut_chips_need_hardware_evidence_and_raw_labels_remain_searchable(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(ipad_window);
    cx.simulate_resize(size(px(1194.), px(834.)));
    view.update(cx, |view, _| view.config.ui.size = 8.);
    open_tab(&view, cx);
    view.read_with(cx, |view, _| {
        let entry = view
            .menu
            .palette
            .as_ref()
            .unwrap()
            .selected_entry()
            .unwrap();
        assert!(matches!(entry.action, Action::Native(Command::Tab)));
        assert_eq!(entry.detail, "⌘⇧P");
    });
    assert!(cx.debug_bounds("ipad-kbd").is_none());
    view.update_in(cx, |view, window, cx| view.dismiss_menu(window, cx));
    cx.simulate_keystrokes("escape");
    open_tab(&view, cx);
    assert!(cx.debug_bounds("ipad-kbd").is_some());
    assert!(cx.debug_bounds("palette-row-0").unwrap().size.height >= px(44.));
    cx.simulate_window_visual_viewport_change(
        cx.window_handle(),
        Bounds::new(point(px(0.), px(0.)), size(px(1194.), px(534.))),
    );
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(cx.debug_bounds("ipad-kbd").is_none());
}

#[gpui::test]
fn an_open_palette_refreshes_its_style_without_changing_its_query(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(fixture_window);
    view.update_in(cx, |view, window, cx| {
        view.config.shortcut_style = ShortcutStyle::Ipad;
        view.open_palette(Filter::Commands, window, cx);
        view.filter_palette("New tab", cx);
    });
    cx.run_until_parked();
    view.update_in(cx, |view, window, cx| {
        view.config.shortcut_style = ShortcutStyle::HerdrPrefix;
        view.refresh_palette(window, cx);
    });
    cx.run_until_parked();
    view.read_with(cx, |view, _| {
        let palette = view.menu.palette.as_ref().unwrap();
        assert_eq!(palette.query, "New tab");
        let entry = palette
            .entries
            .iter()
            .find(|entry| matches!(entry.action, Action::Native(Command::Tab)))
            .unwrap();
        assert_eq!(entry.detail, "⌃B c");
        assert_eq!(view.shortcut_hint(Command::Palette), None);
    });
}

#[test]
fn ipad_chips_are_data_and_palette_prose_is_translated() {
    use crate::i18n::leak_tests::{assert_translated, draw, under_en_xa};
    under_en_xa("iPad palette shortcut chips", |cx, recorder| {
        let _ipad = crate::ipad_ui::ipad_layout();
        let (view, cx) = cx.add_window_view(ipad_window);
        cx.simulate_resize(size(px(1194.), px(834.)));
        draw(cx);
        cx.simulate_keystrokes("escape");
        recorder.take();
        open_tab(&view, cx);
        draw(cx);
        assert!(cx.debug_bounds("ipad-kbd").is_some());
        assert_translated(
            "iPad palette",
            &recorder.take(),
            &["⌘⇧P".into(), "cmd-shift-p".into()],
        );
    });
}
