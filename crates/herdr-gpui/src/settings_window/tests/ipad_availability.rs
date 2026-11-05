//! Platform policy must agree across navigation, rows, search and direct opens.
use super::*;
use gpui::{px, size};

#[gpui::test]
fn ipad_hides_unusable_controls_and_search_without_changing_config(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(skill_fixture);
    cx.simulate_resize(size(px(1200.), px(5000.)));
    view.update(cx, |view, _| view.config.features.acp_chats = true);
    for (section, hidden, retained) in [
        (
            Section::General,
            &[
                "general-ports",
                "general-checkpoints",
                "general-copy-on-select",
                "general-merged-worktrees",
                "general-browser-skill",
                "general-update-beta",
            ][..],
            &[
                "general-language",
                "general-smooth-scroll",
                "general-copied-toast",
                "general-chat-launcher",
            ][..],
        ),
        (
            Section::StatusBar,
            &["status-usage", "status-system-load", "status-gpu"][..],
            &[
                "status-keep-awake",
                "status-theme",
                "status-network-quality",
            ][..],
        ),
        (
            Section::Tabs,
            &[
                "tabs-herdr-2",
                "tabs-position",
                "tabs-hide-single",
                "tabs-pane-history",
            ][..],
            &["tabs-layout", "tabs-confirm-tab"][..],
        ),
        (
            Section::Notifications,
            &["notif-delivery", "notif-sound"][..],
            &["notif-finished", "notif-volume"][..],
        ),
        (
            Section::Code,
            &["settings-code-start"][..],
            &["settings-code-address"][..],
        ),
        (
            Section::Configuration,
            &["config-shared"][..],
            &["config-reload"][..],
        ),
    ] {
        view.update(cx, |view, _| view.section = section);
        cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
        for selector in hidden {
            assert!(cx.debug_bounds(selector).is_none(), "hidden {selector}");
        }
        if section == Section::StatusBar {
            assert!(cx.debug_bounds("status-keep-awake-scope-0").is_some());
            assert!(cx.debug_bounds("status-keep-awake-scope-1").is_none());
        }
        for selector in retained {
            assert!(cx.debug_bounds(selector).is_some(), "retained {selector}");
        }
    }
    view.update_in(cx, |view, window, cx| {
        view.search_results = Some((0..catalog::CATALOG.len()).collect());
        let results = view.visible_results();
        assert!(
            results
                .iter()
                .any(|index| catalog::CATALOG[*index].id == "fonts-terminal")
        );
        assert!(
            results
                .iter()
                .any(|index| catalog::CATALOG[*index].id == "theme-mode")
        );
        for id in [
            "general-browser-skill",
            "general-update-beta",
            "plugins-values",
            "remote-enabled",
        ] {
            let index = catalog::CATALOG
                .iter()
                .position(|entry| entry.id == id)
                .unwrap();
            assert!(!results.contains(&index), "search hid {id}");
            view.open_result(index, window, cx);
            assert_eq!(view.section, Section::Configuration);
        }
        for section in [Section::Plugins, Section::RemoteControl] {
            view.select_section(section, window, cx);
            assert_eq!(view.section, Section::Configuration);
        }
        // Hiding is presentation only, including values that were on already.
        assert!(view.config.show_listening_ports);
        assert_eq!(
            view.config.updates.channel,
            Config::default().updates.channel
        );
    });
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
    for section in [Section::Plugins, Section::RemoteControl] {
        assert!(cx.debug_bounds(section.message()).is_none());
    }
}

#[gpui::test]
fn desktop_keeps_settings_and_search(cx: &mut TestAppContext) {
    let _desktop = crate::ipad_ui::desktop();
    let (view, cx) = cx.add_window_view(skill_fixture);
    cx.simulate_resize(size(px(1200.), px(5000.)));
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
    for selector in [
        "general-browser-skill",
        "general-update-beta",
        "general-ports",
    ] {
        assert!(cx.debug_bounds(selector).is_some(), "desktop {selector}");
    }
    view.update(cx, |view, _| {
        view.search_results = Some((0..catalog::CATALOG.len()).collect());
        for id in [
            "general-browser-skill",
            "general-update-beta",
            "remote-enabled",
            "plugins-values",
        ] {
            assert!(
                view.visible_results()
                    .iter()
                    .any(|index| catalog::CATALOG[*index].id == id)
            );
        }
    });
}
