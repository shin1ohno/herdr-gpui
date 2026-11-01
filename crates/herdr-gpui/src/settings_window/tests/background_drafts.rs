//! iPadOS saves the open Settings window's drafts when the app goes to the
//! background, without closing it, and waits until they are written.
use super::*;
use crate::config::LayoutMode;
use std::{cell::Cell, rc::Rc};

struct Fixture {
    view: Entity<SettingsWindow>,
    themes: Arc<Mutex<Vec<String>>>,
    layouts: Arc<Mutex<Vec<LayoutMode>>>,
}

fn open_settings(cx: &mut TestAppContext) -> (Fixture, &mut VisualTestContext) {
    let main = cx.add_window(crate::sidebar::layout_tests::fixture_window);
    let weak = cx.update(|cx| main.update(cx, |_, _, cx| cx.weak_entity()).unwrap());
    let themes = Arc::new(Mutex::new(Vec::new()));
    let layouts = Arc::new(Mutex::new(Vec::new()));
    let (view, cx) = cx.add_window_view({
        let (themes, layouts) = (themes.clone(), layouts.clone());
        move |window, cx| {
            let mut view = SettingsWindow::new(weak, cx);
            view.theme_io = Some(recording_themes(themes, false));
            view.layout_io = Some(recording_layouts(layouts, false));
            window.focus(&view.focus, cx);
            view
        }
    });
    cx.update(|_, cx| {
        cx.default_global::<SettingsWindowHandle>().model = Some(view.downgrade());
    });
    (
        Fixture {
            view,
            themes,
            layouts,
        },
        cx,
    )
}

/// Starts `save_open_drafts` and returns a flag set once its task resolves.
fn save_in_background(cx: &mut VisualTestContext) -> Rc<Cell<bool>> {
    let done = Rc::new(Cell::new(false));
    let flag = done.clone();
    cx.update(|_, cx| {
        let drafts = save_open_drafts(cx);
        cx.spawn(async move |_| {
            drafts.await;
            flag.set(true);
        })
        .detach();
    });
    done
}

/// Runs everything ready, then lets the settle check poll a few times.
fn settle(cx: &mut VisualTestContext) {
    for _ in 0..3 {
        cx.run_until_parked();
        cx.executor().advance_clock(background::SETTLE_POLL);
    }
    cx.run_until_parked();
}

#[gpui::test]
fn saving_open_drafts_without_a_settings_window_resolves_at_once(cx: &mut TestAppContext) {
    let drafts = cx.update(save_open_drafts);
    cx.update(|cx| assert!(cx.try_global::<SettingsWindowHandle>().is_none()));
    let (done, finished) = std::sync::mpsc::sync_channel(1);
    cx.executor()
        .spawn(async move {
            drafts.await;
            done.send(()).unwrap();
        })
        .detach();
    cx.run_until_parked();
    finished.try_recv().unwrap();
}

#[gpui::test]
fn saving_open_drafts_writes_the_whole_chain_and_keeps_the_window(cx: &mut TestAppContext) {
    let (fixture, cx) = open_settings(cx);
    fixture.view.update(cx, |view, cx| {
        choose(view, "Nord", cx);
        view.accept_layout_choice(LayoutMode::Orca, cx);
    });

    let done = save_in_background(cx);
    settle(cx);

    assert!(done.get());
    assert_eq!(*fixture.themes.lock().unwrap(), ["Nord"]);
    assert_eq!(*fixture.layouts.lock().unwrap(), [LayoutMode::Orca]);
    fixture.view.read_with(cx, |view, _| {
        assert!(!view.theme_dirty());
        assert!(view.layout_intent.is_none());
        assert!(view.closing.is_none());
        assert!(view.drafts_settled());
    });
}

#[gpui::test]
fn a_save_already_running_continues_into_the_drafts_and_is_waited_for(cx: &mut TestAppContext) {
    let (fixture, cx) = open_settings(cx);
    fixture.view.update(cx, |view, cx| choose(view, "Nord", cx));
    // Another write is in flight, so "Save now" itself would do nothing.
    fixture.view.update(cx, |view, _| view.saving = true);

    let done = save_in_background(cx);
    settle(cx);

    assert!(!done.get(), "resolved while a write was still running");
    assert!(fixture.themes.lock().unwrap().is_empty());
    fixture.view.read_with(cx, |view, _| assert!(view.flushing));

    // The running write completes as every save does: its completion reaches
    // `finish_flush` through `finish_close`, which starts the theme draft.
    fixture.view.update(cx, |view, cx| {
        view.saving = false;
        view.finish_close(cx);
    });
    settle(cx);

    assert!(done.get());
    assert_eq!(*fixture.themes.lock().unwrap(), ["Nord"]);
    fixture.view.read_with(cx, |view, _| {
        assert!(!view.theme_dirty());
        assert!(!view.flushing);
        assert!(view.closing.is_none());
    });
}

#[gpui::test]
fn a_quitting_window_leaves_its_drafts_to_the_quit(cx: &mut TestAppContext) {
    let (fixture, cx) = open_settings(cx);
    fixture.view.update(cx, |view, cx| {
        choose(view, "Nord", cx);
        view.quitting = true;
    });

    let done = save_in_background(cx);
    cx.run_until_parked();

    assert!(done.get());
    assert!(fixture.themes.lock().unwrap().is_empty());
    fixture
        .view
        .read_with(cx, |view, _| assert!(!view.flushing));
}
