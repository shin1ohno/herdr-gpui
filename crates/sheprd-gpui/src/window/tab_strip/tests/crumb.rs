use super::*;
use crate::sidebar::{
    SidebarMode,
    layout_tests::{fixture_window, snapshot},
};

#[gpui::test]
fn crumb_leads_tabs_and_opens_the_workspace_picker_only_in_rail(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        let mut listing = snapshot(2);
        listing.focused_workspace_id = Some("w0".into());
        listing.focused_tab_id = Some("t0".into());
        view.live.snapshot = Some(Arc::new(listing));
        view.sidebar_visible = false;
        view
    });
    cx.simulate_resize(size(px(1194.), px(834.)));
    draw(cx);
    let crumb = cx.debug_bounds("ipad-rail-crumb").unwrap();
    let tab = cx.debug_bounds("tab-t0").unwrap();
    assert!(crumb.size.height >= px(44.) && crumb.size.width >= px(44.));
    assert!(crumb.right() <= tab.left(), "{crumb:?} precedes {tab:?}");
    cx.simulate_click(crumb.center(), Modifiers::default());
    view.read_with(cx, |view, _| {
        assert_eq!(view.menu.page, Some(crate::menu::Page::Palette))
    });
    cx.update(|window, cx| {
        view.update(cx, |view, cx| {
            view.dismiss_menu(window, cx);
            view.sidebar_visible = true;
            cx.notify();
        })
    });
    draw(cx);
    assert!(cx.debug_bounds("ipad-rail-crumb").is_none());
    view.update(cx, |view, cx| {
        view.sidebar_visible = false;
        view.settings.shared = Some(
            crate::herdr_settings::Settings::parse_text(
                "[ui]\nsidebar_collapsed_mode = 'hidden'\n",
            )
            .unwrap(),
        );
        cx.notify();
    });
    draw(cx);
    assert!(cx.debug_bounds("ipad-rail-crumb").is_none());
    view.update(cx, |view, cx| {
        view.settings.shared = None;
        cx.notify();
    });
    let _desktop = crate::ipad_ui::desktop();
    draw(cx);
    assert!(cx.debug_bounds("ipad-rail-crumb").is_none());
}

#[test]
fn crumb_labels_the_focused_workspace_with_an_ellipsis() {
    use crate::i18n::leak_tests::{draw, under_en_xa};
    under_en_xa("rail crumb label", |cx, recorder| {
        let _ipad = crate::ipad_ui::ipad_layout();
        let label = "a long focused workspace label which must be truncated in the leading crumb";
        let (view, cx) = cx.add_window_view(|window, cx| {
            let mut view = fixture_window(window, cx);
            let mut listing = snapshot(2);
            listing.focused_workspace_id = Some("w0".into());
            listing.focused_tab_id = Some("t0".into());
            listing.workspaces[0].label = label.into();
            view.live.snapshot = Some(Arc::new(listing));
            view.sidebar_visible = false;
            view
        });
        cx.simulate_resize(size(px(1194.), px(834.)));
        draw(cx);
        assert!(cx.debug_bounds("ipad-rail-crumb-label").unwrap().size.width <= px(180.));
        let seen = recorder.take();
        assert!(seen.iter().any(|line| line == label));
        assert!(seen.iter().any(|line| {
            line.strip_suffix('…')
                .is_some_and(|stem| !stem.is_empty() && label.starts_with(stem))
        }));
        view.update(cx, |view, cx| {
            let listing = Arc::make_mut(view.live.snapshot.as_mut().unwrap());
            listing.focused_workspace_id = None;
            for workspace in &mut listing.workspaces {
                workspace.focused = false;
            }
            cx.notify();
        });
        draw(cx);
        assert!(cx.debug_bounds("ipad-rail-crumb").is_none());
    });
}

#[gpui::test]
fn rail_status_hint_expands_and_disappears_with_the_rail(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = cx.add_window_view(|window, cx| {
        let mut view = fixture_window(window, cx);
        view.live.snapshot = Some(Arc::new(snapshot(1)));
        view.sidebar_visible = false;
        view
    });
    for width in [1194., 834., 500., 320.] {
        cx.simulate_resize(size(px(width), px(834.)));
        draw(cx);
        let hint = cx.debug_bounds("ipad-rail-hint").unwrap();
        let bar = cx.debug_bounds("connection-status").unwrap();
        assert_eq!(hint.size.height, px(44.));
        assert!(hint.size.width >= px(44.), "{width}: {hint:?}");
        assert!(
            hint.left() >= bar.left() && hint.right() <= bar.right(),
            "{width}: {hint:?}"
        );
        assert!(cx.debug_bounds("ipad-rail-hint-shortcut").is_some());
    }
    let hint = cx.debug_bounds("ipad-rail-hint").unwrap();
    view.read_with(cx, |view, _| {
        assert_eq!(view.sidebar_mode(), SidebarMode::Rail)
    });
    cx.simulate_click(hint.center(), Modifiers::default());
    view.read_with(cx, |view, _| assert!(view.sidebar_visible));
    draw(cx);
    assert!(cx.debug_bounds("ipad-rail-hint").is_none());
}

#[test]
fn rail_status_hint_is_translated_with_both_shortcut_styles() {
    use crate::i18n::leak_tests::{assert_translated, draw, uncut, under_en_xa};
    under_en_xa("rail status hint", |cx, recorder| {
        let _ipad = crate::ipad_ui::ipad_layout();
        let (view, cx) = cx.add_window_view(|window, cx| {
            let mut view = fixture_window(window, cx);
            view.live.snapshot = Some(Arc::new(snapshot(1)));
            view.sidebar_visible = false;
            view
        });
        cx.simulate_resize(size(px(1194.), px(834.)));
        draw(cx);
        let mut seen = recorder.take();
        let mut data = vec![
            "Local".into(),
            "local".into(),
            "herdr".into(),
            "Claude Code".into(),
            "agent".into(),
            "tab 1".into(),
            "tab 2".into(),
            env!("HERDR_BUILD_BRANCH").to_owned(),
        ];
        let snapshot = snapshot(1);
        data.extend(
            snapshot
                .agents
                .iter()
                .map(|agent| crate::sidebar::agent_label(agent, &snapshot)),
        );
        for style in [
            crate::config::ShortcutStyle::Ipad,
            crate::config::ShortcutStyle::HerdrPrefix,
        ] {
            view.update(cx, |view, cx| {
                view.config.shortcut_style = style;
                for command in [
                    Command::Tab,
                    Command::SplitRight,
                    Command::ToggleSidebar,
                    Command::Palette,
                    Command::ClosePane,
                    Command::Keybinds,
                ] {
                    data.extend(view.shortcut_hint(command));
                }
                cx.notify();
            });
            draw(cx);
            seen.extend(recorder.take());
            assert!(cx.debug_bounds("ipad-rail-hint-shortcut").is_some());
        }
        let cuts: Vec<_> = seen
            .iter()
            .filter(|line| {
                line.strip_suffix('…')
                    .is_some_and(|stem| data.iter().any(|name| name.starts_with(stem)))
            })
            .cloned()
            .collect();
        data.extend(cuts);
        assert_translated("rail status hint", &uncut(seen), &data);
    });
}
