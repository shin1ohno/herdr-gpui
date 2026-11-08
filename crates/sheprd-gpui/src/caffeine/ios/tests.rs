//! iPad state transitions stand in for the UIKit idle timer.
#![allow(clippy::unwrap_used)]
use super::*;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Default)]
struct Reports(Rc<RefCell<Vec<crate::Error>>>);

impl Reports {
    fn sink(&self) -> impl FnOnce(crate::Error, &mut App) + 'static {
        let errors = self.0.clone();
        move |error, _| errors.borrow_mut().push(error)
    }
}

#[gpui::test]
fn system_scope_is_rejected_without_changing_the_idle_timer(cx: &mut gpui::TestAppContext) {
    let reports = Reports::default();
    cx.update(|cx| {
        set_scope(AwakeScope::System, cx);
        hold_with(
            cx,
            Mode::Indefinite,
            || panic!("system hold reached UIKit"),
            reports.sink(),
        );
        assert_eq!(cup(cx), Cup::Off);
        assert_eq!(changes(cx), 1);
    });
    cx.run_until_parked();
    assert!(matches!(
        reports.0.borrow().as_slice(),
        [crate::Error::Caffeine(Error::UnsupportedScope)]
    ));
}

#[gpui::test]
fn a_display_hold_restores_the_previous_idle_timer_setting(cx: &mut gpui::TestAppContext) {
    let reports = Reports::default();
    for previous in [false, true] {
        cx.update(|cx| {
            hold_with(cx, Mode::Indefinite, || Ok(previous), reports.sink());
            assert_eq!(cup(cx), Cup::On(Mode::Indefinite));
            release_with(
                cx,
                |restored| {
                    assert_eq!(restored, previous);
                    Ok(())
                },
                reports.sink(),
            );
            assert_eq!(cup(cx), Cup::Off);
        });
    }
    cx.run_until_parked();
    assert!(reports.0.borrow().is_empty());
    assert_eq!(cx.update(|cx| changes(cx)), 4);
}

#[gpui::test]
fn a_failed_release_keeps_the_hold_visible(cx: &mut gpui::TestAppContext) {
    let reports = Reports::default();
    cx.update(|cx| {
        hold_with(cx, Mode::Indefinite, || Ok(false), reports.sink());
        release_with(cx, |_| Err(Error::MainThread.into()), reports.sink());
        assert_eq!(cup(cx), Cup::On(Mode::Indefinite));
        assert_eq!(changes(cx), 1);
        release_with(cx, |_| Ok(()), reports.sink());
        assert_eq!(cup(cx), Cup::Off);
    });
    cx.run_until_parked();
    assert!(matches!(
        reports.0.borrow().as_slice(),
        [crate::Error::Caffeine(Error::MainThread)]
    ));
}

#[gpui::test]
fn a_scope_change_during_a_hold_affects_only_the_next_hold(cx: &mut gpui::TestAppContext) {
    let reports = Reports::default();
    cx.update(|cx| {
        hold_with(cx, Mode::Indefinite, || Ok(false), reports.sink());
        set_scope(AwakeScope::System, cx);
        assert_eq!(scope(cx), AwakeScope::Display);
        release_with(cx, |_| Ok(()), reports.sink());
        assert_eq!(scope(cx), AwakeScope::System);
        hold_with(
            cx,
            Mode::Indefinite,
            || panic!("unsupported scope reached UIKit"),
            reports.sink(),
        );
        assert_eq!(cup(cx), Cup::Off);
    });
    cx.run_until_parked();
    assert!(matches!(
        reports.0.borrow().as_slice(),
        [crate::Error::Caffeine(Error::UnsupportedScope)]
    ));
}
