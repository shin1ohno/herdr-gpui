//! iPad display holds use UIKit's process-wide idle timer. A system-only
//! assertion is unavailable: iPadOS controls suspension independently of it.

use super::*;
use objc2::MainThreadMarker;
use objc2_ui_kit::UIApplication;

pub(crate) fn toggle(cx: &mut App, report: impl FnOnce(crate::Error, &mut App) + 'static) {
    match cup(cx) {
        Cup::Pending => {}
        Cup::Off => hold(cx, Mode::Indefinite, report),
        Cup::On(_) => release(cx, report),
    }
}

pub(crate) fn toggle_until_agents_finish(
    cx: &mut App,
    report: impl FnOnce(crate::Error, &mut App) + 'static,
) {
    let caffeine = cx.default_global::<Caffeine>();
    match caffeine.cup {
        Cup::Pending => {}
        Cup::On(Mode::UntilAgentsFinish) => release(cx, report),
        _ if caffeine.working.is_empty() => {
            cx.defer(move |cx| report(Error::NoWorkingAgents.into(), cx));
        }
        Cup::On(Mode::Indefinite) => {
            caffeine.cup = Cup::On(Mode::UntilAgentsFinish);
            cx.refresh_windows();
        }
        Cup::Off => hold(cx, Mode::UntilAgentsFinish, report),
    }
}

fn hold(cx: &mut App, mode: Mode, report: impl FnOnce(crate::Error, &mut App) + 'static) {
    hold_with(cx, mode, disable_idle_timer, report);
}

fn hold_with(
    cx: &mut App,
    mode: Mode,
    disable: impl FnOnce() -> Result<bool>,
    report: impl FnOnce(crate::Error, &mut App) + 'static,
) {
    let caffeine = cx.default_global::<Caffeine>();
    let scope = caffeine.scope;
    caffeine.cup = Cup::Pending;
    caffeine.held = scope;
    let result = display_scope(scope).and_then(|()| disable());
    let caffeine = cx.default_global::<Caffeine>();
    caffeine.changes = caffeine.changes.wrapping_add(1);
    match result {
        Ok(previous) => {
            caffeine.previous_idle_timer_disabled = Some(previous);
            caffeine.held = scope;
            caffeine.cup = Cup::On(mode);
            cx.refresh_windows();
            settle(cx, report);
        }
        Err(error) => {
            caffeine.cup = Cup::Off;
            cx.refresh_windows();
            cx.defer(move |cx| report(error, cx));
        }
    }
}

pub(super) fn release(cx: &mut App, report: impl FnOnce(crate::Error, &mut App) + 'static) {
    release_with(cx, restore_idle_timer, report);
}

fn release_with(
    cx: &mut App,
    restore: impl FnOnce(bool) -> Result<()>,
    report: impl FnOnce(crate::Error, &mut App) + 'static,
) {
    let previous = cx.default_global::<Caffeine>().previous_idle_timer_disabled;
    if let Some(previous) = previous
        && let Err(error) = restore(previous)
    {
        // A failed release must keep reporting the active hold.
        cx.defer(move |cx| report(error, cx));
        return;
    }
    let caffeine = cx.default_global::<Caffeine>();
    caffeine.previous_idle_timer_disabled = None;
    caffeine.cup = Cup::Off;
    caffeine.changes = caffeine.changes.wrapping_add(1);
    cx.refresh_windows();
}

/// iPadOS cannot prevent system suspension while allowing the display to lock.
fn display_scope(scope: AwakeScope) -> Result<()> {
    match scope {
        AwakeScope::Display => Ok(()),
        AwakeScope::System => Err(Error::UnsupportedScope.into()),
    }
}

fn disable_idle_timer() -> Result<bool> {
    let mtm = MainThreadMarker::new().ok_or(Error::MainThread)?;
    let application = UIApplication::sharedApplication(mtm);
    let previous = application.isIdleTimerDisabled();
    application.setIdleTimerDisabled(true);
    Ok(previous)
}

fn restore_idle_timer(previous: bool) -> Result<()> {
    let mtm = MainThreadMarker::new().ok_or(Error::MainThread)?;
    UIApplication::sharedApplication(mtm).setIdleTimerDisabled(previous);
    Ok(())
}

#[cfg(test)]
mod tests;
