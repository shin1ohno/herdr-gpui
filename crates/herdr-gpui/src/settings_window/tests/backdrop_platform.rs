use super::*;

#[gpui::test]
fn ipad_theme_shows_supported_settings_without_unusable_backdrop_actions(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(skill_fixture);
    cx.simulate_resize(size(px(960.), px(5000.)));
    for image in [None, Some(std::path::PathBuf::from("/missing-picture.png"))] {
        let configured = crate::config::Backdrop {
            opacity: 0.2,
            blur: true,
            image,
            ..Default::default()
        };
        view.update(cx, |view, cx| {
            view.section = Section::Theme;
            view.config.backdrop = configured.clone();
            cx.notify();
        });
        cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
        assert!(cx.debug_bounds("settings-backdrop-ipad").is_some());
        for hidden in [
            "settings-backdrop-solid",
            "settings-backdrop-translucent",
            "settings-backdrop-picture",
            "settings-backdrop-ruler",
            "settings-backdrop-blur",
            "settings-backdrop-change",
            "settings-backdrop-remove",
        ] {
            assert!(cx.debug_bounds(hidden).is_none(), "unavailable {hidden}");
        }
        for retained in ["theme-mode", "theme-contrast", "theme-padding-left"] {
            assert!(cx.debug_bounds(retained).is_some(), "supported {retained}");
        }
        view.read_with(cx, |view, _| assert_eq!(view.config.backdrop, configured));
    }
    view.update(cx, |view, _| {
        view.search_results = Some((0..catalog::CATALOG.len()).collect());
        let visible = view.visible_results();
        assert!(
            visible
                .iter()
                .any(|index| catalog::CATALOG[*index].id == "theme-mode")
        );
        assert!(
            !visible
                .iter()
                .any(|index| catalog::CATALOG[*index].id == "theme-backdrop")
        );
    });
}

#[gpui::test]
fn desktop_keeps_native_backdrop_controls_and_search(cx: &mut TestAppContext) {
    let _desktop = crate::ipad_ui::desktop();
    let (view, cx) = cx.add_window_view(skill_fixture);
    cx.simulate_resize(size(px(960.), px(5000.)));
    view.update(cx, |view, cx| {
        view.section = Section::Theme;
        view.config.backdrop.opacity = 0.35;
        view.config.backdrop.blur = true;
        cx.notify();
    });
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
    assert!(cx.debug_bounds("settings-backdrop-ipad").is_none());
    for retained in [
        "settings-backdrop-solid",
        "settings-backdrop-translucent",
        "settings-backdrop-picture",
        "settings-backdrop-ruler",
        "settings-backdrop-blur",
    ] {
        assert!(cx.debug_bounds(retained).is_some(), "desktop {retained}");
    }
    view.update(cx, |view, _| {
        view.search_results = Some((0..catalog::CATALOG.len()).collect());
        assert!(
            view.visible_results()
                .iter()
                .any(|index| catalog::CATALOG[*index].id == "theme-backdrop")
        );
    });
}
