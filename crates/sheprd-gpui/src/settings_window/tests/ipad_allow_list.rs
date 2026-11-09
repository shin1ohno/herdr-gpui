use super::*;
use gpui::{px, size};

#[gpui::test]
fn ipad_rows_are_hidden_in_pages_and_search_but_desktop_keeps_them(cx: &mut TestAppContext) {
    let (view, cx) = cx.add_window_view(skill_fixture);
    cx.simulate_resize(size(px(1200.), px(5000.)));
    for ipad in [false, true] {
        let _ipad = ipad.then(crate::ipad_ui::ipad_layout);
        assert_eq!(availability::row("unclassified-future-setting"), !ipad);
        for (section, hidden, retained) in [
            (Section::Sidebar, "sidebar-automations", "sidebar-layout"),
            (Section::Code, "code-mode", "settings-code-test"),
        ] {
            view.update_in(cx, |view, window, cx| {
                view.select_section(section, window, cx);
                view.search_results = Some((0..catalog::CATALOG.len()).collect());
                assert_eq!(availability::row(hidden), !ipad);
                for index in view.visible_results() {
                    if ipad {
                        assert_ne!(catalog::CATALOG[index].id, hidden);
                    }
                }
                view.search_results = None;
            });
            cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
            assert_eq!(cx.debug_bounds(hidden).is_some(), !ipad, "{hidden}");
            if section == Section::Sidebar {
                assert!(cx.debug_bounds(retained).is_some());
                assert!(cx.debug_bounds("sidebar-activity-bar-toggle").is_some());
            }
        }
    }
}

#[test]
fn ipad_allow_list_preserves_reviewed_catalog_rows() {
    let _ipad = crate::ipad_ui::ipad_layout();
    for entry in catalog::CATALOG {
        let excluded = matches!(entry.section, Section::Plugins | Section::RemoteControl)
            || [
                // keep-sorted start
                "cloud-accounts",
                "cloud-setup",
                "code-mode",
                "config-shared",
                "general-browser-skill",
                "general-checkpoints",
                "general-copy-on-select",
                "general-launch-at-login",
                "general-merged-worktrees",
                "general-ports",
                "general-update-beta",
                "notif-delivery",
                "notif-sound",
                "sidebar-automations",
                "sidebar-gpu-load",
                "status-gpu",
                "status-system-load",
                "status-usage",
                "tabs-herdr-2",
                "tabs-hide-single",
                "tabs-pane-history",
                "tabs-position",
                "theme-backdrop",
                "theme-follow",
                "theme-source",
                // keep-sorted end
            ]
            .contains(&entry.id);
        assert_eq!(availability::row(entry.id), !excluded, "{}", entry.id);
    }
}
