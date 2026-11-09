use super::*;
use crate::settings_window::device_key::{Line, Operation};

const KEY: &str = "restrict ecdsa-sha2-nistp256 AAAAsheet herdr-ipad-test";

fn open_sheet(
    owner: &Entity<SheprdWindow>,
    cx: &mut VisualTestContext,
) -> (SheetFixture, Arc<AtomicUsize>) {
    open_sheet_at(owner, None, cx)
}

/// Opens a fresh sheet as `open_at` does: the open, then any deep link's
/// selection, each deferred in that order.
fn open_sheet_at(
    owner: &Entity<SheprdWindow>,
    section: Option<Section>,
    cx: &mut VisualTestContext,
) -> (SheetFixture, Arc<AtomicUsize>) {
    let reads = Arc::new(AtomicUsize::new(0));
    let calls = reads.clone();
    let writes = Arc::new(Mutex::new(Vec::new()));
    let recorded = writes.clone();
    let source = owner.downgrade();
    // Exercise the fresh-model path; Fixture::prepare would retain an editor
    // and skip the opener's initializer. Deferred, as `settings_window::open`
    // does, so the owner's window is not already on the stack.
    cx.update(|_, cx| {
        cx.defer(move |cx| {
            open_with(source, cx, move |view, _, _| {
                view.theme_io = Some(recording_themes(recorded, false));
                view.config.features.acp_chats = true;
                view.ipad_key.io = Some(Arc::new(move |operation| {
                    assert_eq!(operation, Operation::Read);
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(KEY.into())
                }));
                assert!(matches!(view.ipad_key.line, Line::Loading));
            });
        });
        if let Some(section) = section {
            cx.defer(move |cx| crate::settings_window::sheet::select_open_section(section, cx));
        }
    });
    cx.run_until_parked();
    let model = cx.update(|_, cx| {
        cx.global::<SettingsWindowHandle>()
            .model
            .as_ref()
            .unwrap()
            .upgrade()
            .unwrap()
    });
    (SheetFixture { model, writes }, reads)
}

#[gpui::test]
fn opening_a_fresh_sheet_reads_generals_key_once(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (owner, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    cx.simulate_resize(size(px(1194.), px(834.)));
    let (fixture, reads) = open_sheet(&owner, cx);
    cx.run_until_parked();
    fixture.model.read_with(cx, |view, _| {
        assert_eq!(view.section, Section::General);
        assert!(matches!(&view.ipad_key.line, Line::Ready(line) if line == KEY));
    });
    assert_eq!(reads.load(Ordering::SeqCst), 1);
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
    assert!(cx.debug_bounds("settings-body").is_some());
    assert!(cx.debug_bounds("settings-chat-env").is_some());
}

#[gpui::test]
fn opening_a_fresh_compact_sheet_loads_the_key_but_keeps_the_section_list(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (owner, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    cx.simulate_resize(size(px(320.), px(834.)));
    let (fixture, reads) = open_sheet(&owner, cx);
    cx.run_until_parked();
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
    fixture.model.read_with(cx, |view, _| {
        assert_eq!(view.section, Section::General);
        assert!(matches!(&view.ipad_key.line, Line::Ready(line) if line == KEY));
        assert!(!view.sheet.page);
        assert!(view.viewport_width < crate::settings_window::sheet::COMPACT_BELOW);
    });
    assert_eq!(reads.load(Ordering::SeqCst), 1);
    assert!(cx.debug_bounds("settings-sections").is_some());
    assert!(cx.debug_bounds("settings-body").is_none());
}

/// A deep link's selection runs after the open, so the compact sheet's
/// reset to the list must not hide the page it links to.
#[gpui::test]
fn a_deep_link_opens_its_page_in_a_fresh_compact_sheet(cx: &mut TestAppContext) {
    let _ipad = crate::ipad_ui::ipad_layout();
    let (owner, cx) = cx.add_window_view(crate::sidebar::layout_tests::fixture_window);
    cx.simulate_resize(size(px(320.), px(834.)));
    let (fixture, _) = open_sheet_at(&owner, Some(Section::Tabs), cx);
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
    fixture.model.read_with(cx, |view, _| {
        assert_eq!(view.section, Section::Tabs);
        assert!(view.sheet.page);
    });
    assert!(cx.debug_bounds("settings-body").is_some());
}
