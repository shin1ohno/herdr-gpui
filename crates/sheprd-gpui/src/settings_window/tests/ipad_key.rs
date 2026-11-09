//! Settings > General's iPad key: a current grant to copy and a replacement
//! that needs an explicit second click, with Keychain work off the UI thread.
use super::*;
use crate::settings_window::device_key::{Line, Operation};

const OLD: &str = "restrict ecdsa-sha2-nistp256 AAAAold herdr-ipad-test";
const NEW: &str = "restrict ecdsa-sha2-nistp256 AAAAnew herdr-ipad-test";

fn page(
    cx: &mut TestAppContext,
    io: impl Fn(Operation) -> sheprd_client::Result<String> + Send + Sync + 'static,
) -> (Entity<SettingsWindow>, &mut VisualTestContext) {
    let (view, cx) = cx.add_window_view(skill_fixture);
    view.update_in(cx, |view, window, cx| {
        view.ipad_key.io = Some(Arc::new(io));
        view.select_section(Section::General, window, cx);
    });
    cx.simulate_resize(size(px(1200.), px(5000.)));
    cx.run_until_parked();
    draw(cx);
    (view, cx)
}

fn draw(cx: &mut VisualTestContext) {
    cx.update(|window, cx| crate::sidebar::layout_tests::full_draw(window, cx).clear(cx));
}

fn click(cx: &mut VisualTestContext, selector: &'static str) {
    let point = cx.debug_bounds(selector).unwrap().center();
    cx.simulate_click(point, Default::default());
    cx.run_until_parked();
    draw(cx);
}

fn copied(cx: &mut VisualTestContext) -> Option<String> {
    cx.update(|_, cx| cx.read_from_clipboard().and_then(|item| item.text()))
}

#[gpui::test]
fn current_key_copies_exactly_and_replacement_needs_two_clicks(cx: &mut TestAppContext) {
    let replacements = Arc::new(AtomicUsize::new(0));
    let calls = replacements.clone();
    let (view, cx) = page(cx, move |operation| {
        Ok(match operation {
            Operation::Read => OLD.into(),
            Operation::Replace => {
                calls.fetch_add(1, Ordering::SeqCst);
                NEW.into()
            }
        })
    });
    for selector in [
        "general-ipad-key",
        "general-ipad-key-line",
        "general-ipad-key-copy",
        "general-ipad-key-replace",
        "ipad-key-replace",
    ] {
        assert!(cx.debug_bounds(selector).is_some(), "{selector}");
    }
    click(cx, "general-ipad-key-copy");
    assert_eq!(copied(cx).as_deref(), Some(OLD));

    click(cx, "ipad-key-replace");
    assert_eq!(replacements.load(Ordering::SeqCst), 0);
    assert!(cx.debug_bounds("ipad-key-warning").is_some());
    view.read_with(cx, |view, _| assert!(view.ipad_key.confirming));
    click(cx, "ipad-key-cancel");
    view.read_with(cx, |view, _| assert!(!view.ipad_key.confirming));
    assert_eq!(replacements.load(Ordering::SeqCst), 0);

    click(cx, "ipad-key-replace");
    click(cx, "ipad-key-replace");
    assert_eq!(replacements.load(Ordering::SeqCst), 1);
    view.read_with(cx, |view, _| {
        assert!(!view.ipad_key.confirming);
        assert!(!view.ipad_key.replacing);
        assert!(matches!(&view.ipad_key.line, Line::Ready(line) if line == NEW));
    });
    click(cx, "general-ipad-key-copy");
    assert_eq!(copied(cx).as_deref(), Some(NEW));
}

#[gpui::test]
fn replacement_confirmation_is_cleared_by_navigation_and_search(cx: &mut TestAppContext) {
    let replacements = Arc::new(AtomicUsize::new(0));
    let calls = replacements.clone();
    let (view, cx) = page(cx, move |operation| {
        if operation == Operation::Replace {
            calls.fetch_add(1, Ordering::SeqCst);
        }
        Ok(OLD.into())
    });
    click(cx, "ipad-key-replace");
    view.update_in(cx, |view, window, cx| {
        view.select_section(Section::Fonts, window, cx);
        assert!(!view.ipad_key.confirming);
        view.select_section(Section::General, window, cx);
    });
    cx.run_until_parked();
    draw(cx);
    click(cx, "ipad-key-replace");
    view.update(cx, |view, cx| {
        assert!(view.ipad_key.confirming);
        view.set_search_query("SSH", cx);
        assert!(!view.ipad_key.confirming);
        view.set_search_query("", cx);
    });
    draw(cx);
    assert!(cx.debug_bounds("ipad-key-cancel").is_none());
    assert_eq!(replacements.load(Ordering::SeqCst), 0);
}

#[gpui::test]
fn desktop_has_no_key_rows_or_search_results(cx: &mut TestAppContext) {
    let _desktop = crate::ipad_ui::desktop();
    let (view, cx) = page(cx, |_| panic!("desktop must not load an iPad key"));
    assert!(cx.debug_bounds("general-ipad-key").is_none());
    assert!(cx.debug_bounds("general-ipad-key-replace").is_none());
    view.update(cx, |view, cx| {
        view.set_search_query("key", cx);
        assert!(view.search_results.as_ref().is_some_and(|results| {
            results
                .iter()
                .any(|&index| catalog::CATALOG[index].id == "general-ipad-key")
        }));
        assert!(view.visible_results().iter().all(|&index| {
            !matches!(
                catalog::CATALOG[index].id,
                "general-ipad-key" | "general-ipad-key-replace"
            )
        }));
    });
}

#[gpui::test]
fn ipad_catalog_results_open_the_key_rows(cx: &mut TestAppContext) {
    let (view, cx) = page(cx, |_| Ok(OLD.into()));
    view.update(cx, |view, cx| {
        view.set_search_query("key", cx);
        for id in ["general-ipad-key", "general-ipad-key-replace"] {
            assert!(
                view.visible_results()
                    .iter()
                    .any(|&index| { catalog::CATALOG[index].id == id })
            );
        }
    });
    let index = catalog::CATALOG
        .iter()
        .position(|entry| entry.id == "general-ipad-key")
        .unwrap();
    view.update_in(cx, |view, window, cx| view.open_result(index, window, cx));
    cx.run_until_parked();
    draw(cx);
    view.read_with(cx, |view, _| {
        assert_eq!(view.section, Section::General);
        assert!(view.search_results.is_none());
    });
    assert!(cx.debug_bounds("general-ipad-key").is_some());
}

#[gpui::test]
fn a_failed_read_can_be_retried(cx: &mut TestAppContext) {
    let reads = Arc::new(AtomicUsize::new(0));
    let calls = reads.clone();
    let (view, cx) = page(cx, move |operation| {
        assert_eq!(operation, Operation::Read);
        if calls.fetch_add(1, Ordering::SeqCst) == 0 {
            Err(sheprd_client::ssh::Error::Unsupported.into())
        } else {
            Ok(OLD.into())
        }
    });
    view.read_with(cx, |view, _| {
        assert!(matches!(view.ipad_key.line, Line::Failed(_)))
    });
    assert!(cx.debug_bounds("general-ipad-key-copy").is_none());
    click(cx, "ipad-key-retry");
    assert_eq!(reads.load(Ordering::SeqCst), 2);
    click(cx, "general-ipad-key-copy");
    assert_eq!(copied(cx).as_deref(), Some(OLD));
}

#[gpui::test]
fn replacement_failure_keeps_current_line_and_allows_retry(cx: &mut TestAppContext) {
    let replacements = Arc::new(AtomicUsize::new(0));
    let calls = replacements.clone();
    let (view, cx) = page(cx, move |operation| match operation {
        Operation::Read => Ok(OLD.into()),
        Operation::Replace if calls.fetch_add(1, Ordering::SeqCst) == 0 => {
            Err(sheprd_client::ssh::Error::Unsupported.into())
        }
        Operation::Replace => Ok(NEW.into()),
    });
    click(cx, "ipad-key-replace");
    click(cx, "ipad-key-replace");
    view.read_with(cx, |view, _| {
        assert!(view.ipad_key.error.is_some());
        assert!(!view.ipad_key.replacing);
        assert!(matches!(&view.ipad_key.line, Line::Ready(line) if line == OLD));
    });
    assert!(cx.debug_bounds("ipad-key-error").is_some());
    click(cx, "general-ipad-key-copy");
    assert_eq!(copied(cx).as_deref(), Some(OLD));
    click(cx, "ipad-key-replace");
    click(cx, "ipad-key-replace");
    view.read_with(cx, |view, _| assert!(view.ipad_key.error.is_none()));
    click(cx, "general-ipad-key-copy");
    assert_eq!(copied(cx).as_deref(), Some(NEW));
}

#[gpui::test]
fn repeated_confirmation_while_busy_starts_only_one_replacement(cx: &mut TestAppContext) {
    let replacements = Arc::new(AtomicUsize::new(0));
    let calls = replacements.clone();
    let (view, cx) = page(cx, move |operation| {
        if operation == Operation::Replace {
            calls.fetch_add(1, Ordering::SeqCst);
        }
        Ok(NEW.into())
    });
    click(cx, "ipad-key-replace");
    view.update(cx, |view, cx| {
        view.replace_ipad_key(cx);
        assert!(view.ipad_key.replacing);
        view.replace_ipad_key(cx);
        view.replace_ipad_key(cx);
        assert!(!view.ipad_key.confirming);
    });
    cx.run_until_parked();
    assert_eq!(replacements.load(Ordering::SeqCst), 1);
}

#[gpui::test]
fn closing_settings_discards_the_key_confirmation(cx: &mut TestAppContext) {
    let replacements = Arc::new(AtomicUsize::new(0));
    let calls = replacements.clone();
    let (view, cx) = page(cx, move |operation| {
        if operation == Operation::Replace {
            calls.fetch_add(1, Ordering::SeqCst);
        }
        Ok(OLD.into())
    });
    click(cx, "ipad-key-replace");
    view.update_in(cx, |view, window, cx| {
        assert!(view.ipad_key.confirming);
        assert!(view.should_close(window, cx));
        assert!(!view.ipad_key.confirming);
    });
    assert_eq!(replacements.load(Ordering::SeqCst), 0);
}

#[gpui::test]
fn long_public_key_and_confirmation_stay_inside_a_narrow_page(cx: &mut TestAppContext) {
    let line = format!(
        "restrict ecdsa-sha2-nistp256 {} herdr-ipad-test",
        "A".repeat(512)
    );
    let expected = line.clone();
    let (_, cx) = page(cx, move |_| Ok(line.clone()));
    cx.simulate_resize(size(px(680.), px(1200.)));
    draw(cx);
    click(cx, "ipad-key-replace");
    let body = cx.debug_bounds("settings-body").unwrap();
    for selector in [
        "general-ipad-key-line",
        "general-ipad-key-copy",
        "ipad-key-replace",
        "ipad-key-cancel",
        "ipad-key-warning",
    ] {
        let bounds = cx.debug_bounds(selector).unwrap();
        assert!(bounds.left() >= body.left(), "{selector}: {bounds:?}");
        assert!(bounds.right() <= body.right(), "{selector}: {bounds:?}");
    }
    click(cx, "general-ipad-key-copy");
    assert_eq!(copied(cx), Some(expected));
}

#[gpui::test]
fn replacement_finishes_and_invalidates_other_views_after_settings_closes(cx: &mut TestAppContext) {
    let replacements = Arc::new(AtomicUsize::new(0));
    let calls = replacements.clone();
    let (view, cx) = page(cx, move |operation| {
        if operation == Operation::Replace {
            calls.fetch_add(1, Ordering::SeqCst);
        }
        Ok(NEW.into())
    });
    let notifications = Arc::new(AtomicUsize::new(0));
    let notified = notifications.clone();
    let _observer = cx.update(|_, cx| {
        cx.observe_global::<device_key::Revision>(move |_| {
            notified.fetch_add(1, Ordering::SeqCst);
        })
    });
    // Activate the observer before queuing the operation. Do not run the
    // executor again until the window and its external Entity have gone away.
    cx.run_until_parked();
    view.update_in(cx, |view, window, cx| {
        view.replace_ipad_key(cx);
        view.replace_ipad_key(cx);
        assert!(view.ipad_key.replacing);
        window.remove_window();
    });
    let weak = view.downgrade();
    drop(view);
    assert!(
        weak.upgrade().is_some(),
        "reopening must reuse the pending model"
    );
    cx.run_until_parked();
    assert!(weak.upgrade().is_none());
    assert_eq!(replacements.load(Ordering::SeqCst), 1);
    assert_eq!(notifications.load(Ordering::SeqCst), 1);
}

#[test]
fn ipad_key_states_show_only_translated_messages() {
    use crate::i18n::leak_tests::{assert_translated, under_en_xa};
    under_en_xa("Settings iPad key en-XA", |cx, recorder| {
        let (view, cx) = cx.add_window_view(skill_fixture);
        cx.simulate_resize(size(px(1400.), px(5000.)));
        draw(cx);
        assert_translated("iPad key loading", &recorder.take(), &[]);

        view.update(cx, |view, cx| {
            view.ipad_key.line = Line::Ready(OLD.into());
            cx.notify();
        });
        draw(cx);
        assert_translated("iPad key ready", &recorder.take(), &[OLD.into()]);

        click(cx, "general-ipad-key-copy");
        assert_translated("iPad key copied", &recorder.take(), &[OLD.into()]);

        click(cx, "ipad-key-replace");
        assert!(cx.debug_bounds("ipad-key-warning").is_some());
        assert_translated("iPad key warning", &recorder.take(), &[OLD.into()]);

        view.update(cx, |view, cx| {
            view.ipad_key.confirming = false;
            view.ipad_key.replacing = true;
            view.ipad_key.line = Line::Loading;
            cx.notify();
        });
        draw(cx);
        assert_translated("iPad key replacing", &recorder.take(), &[OLD.into()]);

        view.update(cx, |view, cx| {
            view.ipad_key.replacing = false;
            view.ipad_key.line = Line::Ready(NEW.into());
            cx.notify();
        });
        draw(cx);
        assert_translated("iPad key replaced", &recorder.take(), &[NEW.into()]);

        view.update(cx, |view, cx| {
            view.ipad_key.replacing = false;
            view.ipad_key.line = Line::Failed(sheprd_client::ssh::Error::Unsupported.localize());
            cx.notify();
        });
        draw(cx);
        assert_translated("iPad key failure", &recorder.take(), &[]);
    });
}

#[gpui::test]
fn reopening_during_replacement_reuses_the_model_and_displays_the_committed_key(
    cx: &mut TestAppContext,
) {
    let source = cx.add_window(crate::sidebar::layout_tests::fixture_window);
    let (retained, reopened) = cx.update(|cx| {
        let weak = source.update(cx, |_, _, cx| cx.weak_entity()).unwrap();
        open_fixture(weak.clone(), cx);
        let settings = cx.global::<SettingsWindowHandle>().window.unwrap();
        let retained = settings
            .update(cx, |view, window, cx| {
                view.section = Section::General;
                view.ipad_key.line = Line::Ready(OLD.into());
                view.ipad_key.io = Some(Arc::new(|operation| {
                    assert_eq!(operation, Operation::Replace);
                    Ok(NEW.into())
                }));
                view.replace_ipad_key(cx);
                view.replace_ipad_key(cx);
                window.remove_window();
                cx.weak_entity()
            })
            .unwrap();
        open_fixture(weak, cx);
        let reopened = cx.global::<SettingsWindowHandle>().window.unwrap();
        reopened
            .update(cx, |view, _, cx| {
                assert_eq!(cx.entity_id(), retained.entity_id());
                assert!(view.ipad_key.replacing);
            })
            .unwrap();
        (retained, reopened)
    });
    cx.run_until_parked();
    cx.update(|cx| {
        reopened
            .update(cx, |view, window, _| {
                assert!(matches!(&view.ipad_key.line, Line::Ready(line) if line == NEW));
                assert!(!view.ipad_key.replacing);
                window.remove_window();
            })
            .unwrap();
    });
    cx.run_until_parked();
    assert!(retained.upgrade().is_none());
}
