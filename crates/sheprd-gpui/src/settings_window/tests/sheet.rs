//! Embedded rendering is supported; the sheet supplies its own shell geometry.
use super::*;

struct Embedded {
    _source: Entity<SheprdWindow>,
    settings: Entity<SettingsWindow>,
}

impl Render for Embedded {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(
            div()
                .debug_selector(|| "settings-embedded-spike-panel".into())
                .w(px(1000.))
                .h(px(760.))
                .child(self.settings.clone()),
        )
    }
}

#[gpui::test]
fn existing_settings_entity_renders_as_a_child(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (host, cx) = cx.add_window_view(|window, cx| {
        let source = cx.new(|cx| crate::sidebar::layout_tests::fixture_window(window, cx));
        let settings = cx.new(|cx| {
            let mut view = SettingsWindow::new(source.downgrade(), cx);
            view.section = Section::General;
            window.focus(&view.focus, cx);
            view
        });
        Embedded {
            _source: source,
            settings,
        }
    });
    cx.simulate_resize(size(px(1194.), px(834.)));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(
        cx.debug_bounds("settings-embedded-spike-panel")
            .unwrap()
            .size,
        size(px(1000.), px(760.))
    );
    let settings = host.update(cx, |host, _| host.settings.clone());
    assert!(cx.debug_bounds("settings-body").unwrap().size.height > px(0.));
    cx.update(|window, cx| {
        assert_eq!(cx.windows().len(), 1);
        settings.update(cx, |view, cx| {
            assert!(view.focus.is_focused(window));
            // The standalone shell currently reads its containing viewport,
            // so Sheet must replace this with the panel width.
            assert_eq!(view.viewport_width, 1194.);
            view.select_section(Section::Tabs, window, cx);
            assert_eq!(view.section, Section::Tabs);
        });
        window.draw(cx).clear(cx);
    });
    assert!(cx.debug_bounds("settings-body").unwrap().size.width > px(0.));
}

/// Isolated IO behind the production opener, shared with the window-level tests.
pub(crate) struct Fixture {
    pub(crate) model: Entity<SettingsWindow>,
    pub(crate) writes: Arc<Mutex<Vec<String>>>,
}

impl Fixture {
    pub(crate) fn prepare(source: WeakEntity<SheprdWindow>, fail: bool, cx: &mut App) -> Self {
        let writes = Arc::new(Mutex::new(Vec::new()));
        let model = cx.new(|cx| {
            let mut view = SettingsWindow::new(source, cx);
            view.shell = Shell::Sheet;
            view.section = Section::General;
            view.theme_io = Some(recording_themes(writes.clone(), fail));
            view.layout_io = Some(recording_layouts(Default::default(), false));
            view
        });
        cx.default_global::<SettingsWindowHandle>().model = Some(model.downgrade());
        Self { model, writes }
    }

    pub(crate) fn dirty(&self, cx: &mut App) {
        self.model.update(cx, |view, cx| choose(view, "Nord", cx));
    }

    pub(crate) fn focused(&self, window: &Window, cx: &App) -> bool {
        self.model.read(cx).focus.is_focused(window)
    }
}

#[gpui::test]
fn registered_sheet_drafts_save_in_background_without_a_second_window(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (owner, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    let fixture = cx.update(|_, cx| Fixture::prepare(owner.downgrade(), false, cx));
    cx.update(|_, cx| open_at(owner.downgrade(), None, cx));
    cx.run_until_parked();
    cx.update(|_, cx| fixture.dirty(cx));
    let saved = cx.update(|_, cx| save_open_drafts(cx));
    let done = std::rc::Rc::new(std::cell::Cell::new(false));
    let completed = done.clone();
    cx.update(|_, cx| {
        cx.spawn(async move |_| {
            saved.await;
            completed.set(true);
        })
        .detach()
    });
    for _ in 0..4 {
        cx.run_until_parked();
        cx.executor().advance_clock(background::SETTLE_POLL);
    }
    assert!(done.get());
    assert_eq!(*fixture.writes.lock().unwrap(), ["Nord"]);
    owner.read_with(cx, |view, _| {
        assert_eq!(view.menu.page, Some(crate::menu::Page::Settings))
    });
    cx.update(|_, cx| {
        assert_eq!(cx.windows().len(), 1);
        let registry = cx.global::<SettingsWindowHandle>();
        assert!(registry.window.is_none());
        assert_eq!(
            registry.model.as_ref().unwrap().entity_id(),
            fixture.model.entity_id()
        );
    });
}

#[gpui::test]
fn sheet_save_failure_retains_drafts_and_error_then_done_can_retry(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (owner, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    let fixture = cx.update(|_, cx| Fixture::prepare(owner.downgrade(), true, cx));
    cx.update(|_, cx| open_at(owner.downgrade(), None, cx));
    cx.run_until_parked();
    cx.update(|window, cx| {
        fixture.dirty(cx);
        owner.update(cx, |view, cx| view.dismiss_menu(window, cx));
    });
    cx.run_until_parked();
    fixture.model.read_with(cx, |view, _| {
        assert!(view.error.is_some());
        assert!(view.theme_dirty());
        assert!(view.closing.is_none());
    });
    owner.read_with(cx, |view, _| {
        assert_eq!(view.menu.page, Some(crate::menu::Page::Settings))
    });
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        fixture.model.update(cx, |view, _| {
            view.theme_io = Some(recording_themes(fixture.writes.clone(), false))
        });
    });
    assert!(cx.debug_bounds("ipad-settings-sheet-status").is_some());
    cx.update(|window, cx| owner.update(cx, |view, cx| view.dismiss_menu(window, cx)));
    cx.run_until_parked();
    owner.read_with(cx, |view, _| assert!(view.menu.page.is_none()));
    assert_eq!(*fixture.writes.lock().unwrap(), ["Nord", "Nord"]);
}

#[gpui::test]
fn sheet_search_and_deep_links_cannot_select_hidden_sections(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (owner, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    let fixture = cx.update(|_, cx| Fixture::prepare(owner.downgrade(), false, cx));
    cx.update(|_, cx| open_at(owner.downgrade(), Some(Section::RemoteControl), cx));
    cx.run_until_parked();
    fixture
        .model
        .read_with(cx, |view, _| assert_eq!(view.section, Section::General));
    cx.update(|_, cx| {
        fixture.model.update(cx, |view, cx| {
            view.set_search_query("remote", cx);
            view.filter_sheet_search();
            assert!(view.visible_results().iter().all(|index| {
                Section::shown().any(|shown| shown == catalog::CATALOG[*index].section)
            }));
        })
    });
}

#[gpui::test]
fn sheet_search_composition_keeps_escape_and_enter_in_the_field(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (owner, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    cx.simulate_resize(size(px(320.), px(834.)));
    let fixture = cx.update(|_, cx| Fixture::prepare(owner.downgrade(), false, cx));
    cx.update(|_, cx| open_at(owner.downgrade(), None, cx));
    cx.run_until_parked();
    cx.update(|window, cx| {
        fixture.model.update(cx, |view, cx| {
            view.nav_search.update(cx, |input, cx| {
                window.focus(&input.focus, cx);
                input.replace_and_mark_text_in_range(None, "東京", Some(2..2), window, cx);
            });
        });
        window.draw(cx).clear(cx);
    });
    for key in ["escape", "enter", "up", "down", "f6"] {
        fixture.model.read_with(cx, |view, cx| {
            assert_eq!(view.nav_search.read(cx).text(), "東京", "before {key}");
            assert!(view.nav_search.read(cx).is_composing(), "before {key}");
        });
        // Raw key events leave IME commits to the platform. The test helper
        // simulate_keystrokes synthesizes Enter as a newline text replacement.
        cx.simulate_event(KeyDownEvent {
            keystroke: Keystroke::parse(key).unwrap(),
            is_held: false,
            prefer_character_input: false,
        });
    }
    owner.read_with(cx, |view, _| {
        assert_eq!(view.menu.page, Some(crate::menu::Page::Settings))
    });
    fixture.model.read_with(cx, |view, cx| {
        assert_eq!(view.nav_search.read(cx).text(), "東京");
        assert!(view.nav_search.read(cx).is_composing());
    });
    cx.update(|window, cx| {
        assert!(!owner.read(cx).focus.is_focused(window));
        fixture.model.update(cx, |view, cx| {
            view.nav_search.update(cx, |input, cx| {
                input.unmark_text(window, cx);
                input.clear(cx);
            });
        });
    });
    cx.simulate_keystrokes("t a b");
    fixture.model.read_with(cx, |view, cx| {
        assert_eq!(view.nav_search.read(cx).text(), "tab")
    });
    cx.simulate_keystrokes("escape");
    owner.read_with(cx, |view, _| assert!(view.menu.page.is_some()));
    cx.simulate_keystrokes("escape");
    cx.run_until_parked();
    owner.read_with(cx, |view, _| assert!(view.menu.page.is_none()));
}

#[gpui::test]
fn reopening_fences_an_older_deferred_dismissal(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (owner, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    let fixture = cx.update(|_, cx| Fixture::prepare(owner.downgrade(), false, cx));
    cx.update(|_, cx| open_at(owner.downgrade(), None, cx));
    cx.run_until_parked();
    cx.update(|window, cx| {
        fixture.model.update(cx, |view, cx| view.close(window, cx));
        open_at(owner.downgrade(), None, cx);
    });
    cx.run_until_parked();
    owner.read_with(cx, |view, _| {
        assert_eq!(view.menu.page, Some(crate::menu::Page::Settings))
    });
}

#[gpui::test]
fn closing_waits_for_an_in_flight_write_and_then_flushes_the_draft(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (owner, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    let fixture = cx.update(|_, cx| Fixture::prepare(owner.downgrade(), false, cx));
    cx.update(|_, cx| open_at(owner.downgrade(), None, cx));
    cx.run_until_parked();
    cx.update(|window, cx| {
        fixture.dirty(cx);
        fixture.model.update(cx, |view, _| view.saving = true);
        owner.update(cx, |view, cx| view.dismiss_menu(window, cx));
    });
    cx.run_until_parked();
    assert!(fixture.writes.lock().unwrap().is_empty());
    owner.read_with(cx, |view, _| assert!(view.menu.page.is_some()));
    cx.update(|_, cx| {
        fixture.model.update(cx, |view, cx| {
            view.saving = false;
            view.drive_theme_intent(cx);
        })
    });
    cx.run_until_parked();
    assert_eq!(*fixture.writes.lock().unwrap(), ["Nord"]);
    owner.read_with(cx, |view, _| assert!(view.menu.page.is_none()));
}

#[cfg(feature = "cloud")]
#[gpui::test]
fn cloud_deep_link_lands_on_general_in_the_sheet(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (owner, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    let fixture = cx.update(|_, cx| Fixture::prepare(owner.downgrade(), false, cx));
    cx.update(|_, cx| open_cloud(owner.downgrade(), None, cx));
    cx.run_until_parked();
    fixture
        .model
        .read_with(cx, |view, _| assert_eq!(view.section, Section::General));
    assert_eq!(cx.windows().len(), 1);
}

#[gpui::test]
fn a_fresh_sheet_creates_and_registers_the_child_outside_the_source_lease(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (owner, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    let initialized = std::rc::Rc::new(std::cell::Cell::new(0));
    let seen = initialized.clone();
    let source = owner.downgrade();
    cx.update(|_, cx| {
        cx.defer(move |cx| {
            open_with(source, cx, move |_, _, _| seen.set(seen.get() + 1));
        })
    });
    cx.run_until_parked();
    cx.update(|window, cx| {
        let registry = cx.global::<SettingsWindowHandle>();
        assert!(registry.window.is_none());
        let model = registry.model.as_ref().unwrap().upgrade().unwrap();
        assert!(model.read(cx).focus.is_focused(window));
        assert_eq!(model.read(cx).section, Section::General);
        assert_eq!(cx.windows().len(), 1);
        assert_eq!(owner.read(cx).menu.settings.as_ref(), Some(&model));
    });
    assert_eq!(initialized.get(), 1);
}

#[gpui::test]
fn save_errors_stay_visible_above_the_compact_section_list(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (owner, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    cx.simulate_resize(size(px(320.), px(834.)));
    let visible = Bounds::new(point(px(0.), px(0.)), size(px(320.), px(420.)));
    cx.simulate_window_visual_viewport_change(cx.window_handle(), visible);
    let fixture = cx.update(|_, cx| Fixture::prepare(owner.downgrade(), true, cx));
    cx.update(|_, cx| open_at(owner.downgrade(), None, cx));
    cx.run_until_parked();
    cx.update(|window, cx| {
        fixture.dirty(cx);
        owner.update(cx, |view, cx| view.dismiss_menu(window, cx));
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let error = cx.debug_bounds("ipad-settings-sheet-error").unwrap();
    assert!(error.size.height > px(0.));
    assert!(error.top() >= visible.top() && error.bottom() <= visible.bottom());
    assert!(cx.debug_bounds("settings-sections").is_some());
    assert!(cx.debug_bounds("settings-body").is_none());
}

#[gpui::test]
fn the_sheet_follows_its_owners_appearance_without_an_os_window_observer(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (owner, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    let fixture = cx.update(|_, cx| Fixture::prepare(owner.downgrade(), false, cx));
    cx.update(|_, cx| open_at(owner.downgrade(), None, cx));
    cx.run_until_parked();
    owner.update(cx, |view, cx| {
        view.theme.background ^= 0x010101;
        cx.notify();
    });
    cx.run_until_parked();
    cx.update(|_, cx| {
        assert_eq!(fixture.model.read(cx).theme, owner.read(cx).theme);
        assert!(fixture.model.read(cx)._appearance.is_none());
    });
}

#[gpui::test]
fn compact_back_cancels_an_invalid_hex_editor_before_hiding_the_page(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (owner, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    cx.simulate_resize(size(px(320.), px(834.)));
    let fixture = cx.update(|_, cx| Fixture::prepare(owner.downgrade(), false, cx));
    cx.update(|_, cx| open_at(owner.downgrade(), Some(Section::Sidebar), cx));
    cx.run_until_parked();
    cx.update(|window, cx| {
        fixture.model.update(cx, |view, cx| {
            view.config
                .sidebar_style
                .hosts
                .insert("devbox".into(), 0x112233);
            view.begin_hex_edit("devbox", window, cx);
            view.hex_edit
                .as_ref()
                .unwrap()
                .input
                .update(cx, |input, cx| input.set_text_selected("invalid", cx));
        });
        window.draw(cx).clear(cx);
    });
    let back = cx
        .debug_bounds("ipad-settings-sheet-back")
        .unwrap()
        .center();
    cx.simulate_click(back, Modifiers::default());
    fixture.model.read_with(cx, |view, _| {
        assert!(view.hex_edit.is_none());
        assert!(!view.sheet.page);
    });
    cx.update(|window, cx| owner.update(cx, |view, cx| view.dismiss_menu(window, cx)));
    cx.run_until_parked();
    owner.read_with(cx, |view, _| assert!(view.menu.page.is_none()));
}
