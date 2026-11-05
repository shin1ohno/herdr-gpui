use super::*;
use crate::config::ShortcutStyle;
use core::prelude::v1::test;

#[gpui::test]
fn ipad_tabs_keep_touch_bounds_and_chat_marks_without_hover(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = layouts::strip(cx, crate::config::TabLayoutMode::Pills);
    view.update(cx, |view, cx| {
        let snapshot = Arc::make_mut(view.live.snapshot.as_mut().unwrap());
        let agent = &mut snapshot.agents[0];
        agent.tab_id = snapshot.tabs[0].tab_id.clone();
        agent.agent = Some("acp".into());
        agent.display_agent = Some("codex".into());
        cx.notify();
    });
    draw(cx);
    assert_eq!(cx.debug_bounds("tab-w1:t1").unwrap().size.height, px(44.));
    for selector in ["close-tab-w1:t1", "close-tab-plain", "close-tab-zoomy"] {
        assert_eq!(
            cx.debug_bounds(selector).unwrap().size,
            size(px(44.), px(44.)),
            "{selector}"
        );
    }
    assert_eq!(
        cx.debug_bounds("tab-status-w1:t1").unwrap().size,
        size(px(8.), px(8.))
    );
    assert_eq!(
        cx.debug_bounds("tab-chat-icon-w1:t1").unwrap().size,
        size(px(16.), px(16.))
    );
    for selector in ["new-tab", "split-editor", "tab-actions"] {
        assert_eq!(
            cx.debug_bounds(selector).unwrap().size,
            size(px(44.), px(44.)),
            "{selector}"
        );
    }
    view.read_with(cx, |view, _| {
        let snapshot = view.live.snapshot.as_deref().unwrap();
        assert!(matches!(
            herdr_mark(snapshot, &snapshot.tabs[0]),
            Mark::Herdr {
                agent: Some(crate::icons::AgentIcon::Codex),
                chat: true,
                ..
            }
        ));
    });
    cx.simulate_keystrokes("escape");
    draw(cx);
    let measured = cx.update(|window, cx| {
        let view = view.read(cx);
        let snapshot = view.live.snapshot.as_deref().unwrap();
        let tab = &snapshot.tabs[0];
        let mark = view.ipad_herdr_mark(snapshot, tab, &view.ipad_numbered_tabs(cx), window);
        let indicators = view
            .tab_context(
                view.group_slots()[0],
                Indicators::new(
                    None,
                    crate::config::StatusIcons::Dots,
                    false,
                    false,
                    &view.theme,
                ),
                window,
            )
            .indicators;
        view.tab_width(
            &tab.label.clone().into(),
            shape(mark.lead(), tab.zoomed),
            indicators,
            window,
        ) + crate::tab_layout::ipad::extra_width(&mark)
    });
    assert!((cx.debug_bounds("tab-w1:t1").unwrap().size.width - px(measured)).abs() <= px(0.5));
}

#[gpui::test]
fn ipad_hints_follow_workspace_order_and_hardware_evidence(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = four_tabs(cx, false);
    assert!(cx.debug_bounds("tab-shortcut-a").is_none());
    cx.simulate_keystrokes("escape");
    draw(cx);
    assert!(cx.debug_bounds("tab-shortcut-a").is_some());
    for style in [ShortcutStyle::Ipad, ShortcutStyle::HerdrPrefix] {
        view.update_in(cx, |view, window, cx| {
            view.config.shortcut_style = style;
            let group = view.group_slots()[0].id;
            if !view.is_split() {
                view.split_group(group, window, cx);
            }
            let numbered = view.ipad_numbered_tabs(cx);
            assert_eq!(
                numbered,
                ["a", "b", "c", "d"].map(|id| Pick::Herdr(id.into()))
            );
            let snapshot = view.live.snapshot.as_deref().unwrap();
            for (index, tab) in snapshot.tabs.iter().enumerate() {
                let Mark::Herdr {
                    hint: Some(hint), ..
                } = view.ipad_herdr_mark(snapshot, tab, &numbered, window)
                else {
                    panic!("missing hint");
                };
                assert_eq!(
                    hint.text.as_ref(),
                    view.shortcut_hint(Command::TabNumber(u8::try_from(index + 1).unwrap()))
                        .unwrap()
                );
                assert_eq!(hint.style, style);
            }
        });
        draw(cx);
    }
    cx.simulate_window_visual_viewport_change(
        cx.window_handle(),
        Bounds::new(point(px(0.), px(0.)), size(px(1600.), px(300.))),
    );
    draw(cx);
    assert!(cx.debug_bounds("tab-shortcut-a").is_none());
}

#[gpui::test]
fn ipad_hint_numbers_stop_at_nine_and_retain_a_published_prefix(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = four_tabs(cx, false);
    view.update(cx, |view, _| {
        let snapshot = Arc::make_mut(view.live.snapshot.as_mut().unwrap());
        let first = snapshot.tabs[0].clone();
        for index in 5..=10 {
            let mut tab = first.clone();
            tab.tab_id = format!("t{index}");
            tab.focused = false;
            snapshot.tabs.push(tab);
        }
        view.config.shortcut_style = ShortcutStyle::HerdrPrefix;
        view.config.keybindings = crate::keymap::Keymap::with_overrides(
            &Default::default(),
            &Default::default(),
            &crate::keymap::DaemonKeys::from_profile(Some("[keys]\nprefix = 'ctrl+a'\n")).unwrap(),
        )
        .unwrap();
    });
    cx.simulate_keystrokes("escape");
    draw(cx);
    view.update_in(cx, |view, window, cx| {
        let numbered = view.ipad_numbered_tabs(cx);
        assert_eq!(numbered.last(), Some(&Pick::Herdr("t9".into())));
        let snapshot = view.live.snapshot.as_deref().unwrap();
        for (index, tab) in snapshot.tabs.iter().enumerate() {
            let Mark::Herdr { hint, .. } = view.ipad_herdr_mark(snapshot, tab, &numbered, window)
            else {
                panic!("Herdr mark");
            };
            assert_eq!(hint.is_some(), index < 9);
            if let Some(hint) = hint {
                assert!(hint.text.starts_with("⌃A "));
            }
        }
    });
}

#[gpui::test]
fn ipad_split_does_not_create_an_editor_group_and_more_keeps_its_menu(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = four_tabs(cx, false);
    let button = cx.debug_bounds("split-editor").unwrap();
    cx.simulate_click(button.center(), Modifiers::default());
    view.read_with(cx, |view, _| assert!(!view.is_split()));
    let more = cx.debug_bounds("tab-actions").unwrap();
    cx.simulate_click(more.center(), Modifiers::default());
    view.read_with(cx, |view, _| assert!(view.menu.page.is_some()));
}

#[gpui::test]
fn ipad_core_and_page_faces_use_workspace_numbered_hints(cx: &mut TestAppContext) {
    use crate::browser::{Location, Store};
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = four_tabs(cx, false);
    let page = view.update(cx, |view, cx| {
        let scope = crate::browser::scope(&view.endpoints[0]);
        Store::update(cx, |store| {
            store.open(
                scope,
                "w1",
                Some(Location::CoreTerminal {
                    panes: crate::core_terminals::panes::Panes::single(
                        herdr_backend::TerminalId::new("owned-test-core").unwrap(),
                    ),
                    observe: false,
                    remote: None,
                }),
                None,
            )
        })
        .unwrap()
    });
    draw(cx);
    let selector: &'static str = Box::leak(format!("tab-shortcut-{page}").into_boxed_str());
    assert!(cx.debug_bounds(selector).is_none());
    cx.simulate_keystrokes("escape");
    draw(cx);
    assert!(cx.debug_bounds(selector).is_some());
    view.update_in(cx, |view, window, cx| {
        let numbered = view.ipad_numbered_tabs(cx);
        assert_eq!(numbered.last(), Some(&Pick::Page(page)));
        let slot = view.group_slots()[0];
        let indicators = Indicators::new(
            None,
            crate::config::StatusIcons::Dots,
            false,
            false,
            &view.theme,
        );
        let context = view.tab_context(slot, indicators, window);
        let entries = view.touch_browser_tab_entries(slot, None, &context, &numbered, window, cx);
        assert!(entries.iter().any(|(id, _)| *id == page));
    });
    let close: &'static str = Box::leak(format!("close-browser-tab-{page}").into_boxed_str());
    assert_eq!(cx.debug_bounds(close).unwrap().size, size(px(44.), px(44.)));
}

#[gpui::test]
fn ipad_numbering_reuses_hidden_tab_and_shown_tab_policy(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (view, cx) = four_tabs(cx, false);
    view.update(cx, |view, cx| {
        let key = view.browser_key().unwrap();
        view.browser.hidden.hide(key, [Pick::Herdr("b".into())]);
        assert_eq!(
            view.ipad_numbered_tabs(cx),
            ["a", "c", "d"].map(|id| Pick::Herdr(id.into()))
        );
        let snapshot = Arc::make_mut(view.live.snapshot.as_mut().unwrap());
        snapshot.focused_tab_id = Some("b".into());
        for tab in &mut snapshot.tabs {
            tab.focused = tab.tab_id == "b";
        }
        assert_eq!(
            view.ipad_numbered_tabs(cx),
            ["a", "b", "c", "d"].map(|id| Pick::Herdr(id.into()))
        );
    });
}
