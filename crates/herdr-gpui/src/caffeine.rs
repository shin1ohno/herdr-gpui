//! App-wide keep-awake state. Desktop power assertions live on a worker
//! thread; iPad display holds use UIKit's idle timer on the UI thread.
//! Every window observes the same hold and the daemon's agent activity.

use crate::{Result, config::status_bar::AwakeScope};
use gpui::{App, Global, WindowId};
use std::collections::HashSet;

#[cfg(not(target_os = "ios"))]
mod desktop;
mod error;
#[cfg(target_os = "ios")]
mod ios;
mod work;

#[cfg(not(target_os = "ios"))]
use desktop as platform;
#[cfg(target_os = "ios")]
use ios as platform;

pub(crate) use error::Error;
pub(crate) use platform::{toggle, toggle_until_agents_finish};
pub(crate) use work::Work;

/// How long a hold lasts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    /// Until the cup is clicked again.
    Indefinite,
    /// Until no window has a working agent.
    UntilAgentsFinish,
}

/// What the status bar draws for the cup.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Cup {
    #[default]
    Off,
    /// A hold or release is in flight; clicks wait for it.
    Pending,
    On(Mode),
}

#[derive(Default)]
struct Caffeine {
    /// Settled holds and releases fence timers that belong to older state.
    changes: u64,
    cup: Cup,
    /// The scope the active hold started with, until it is released.
    held: AwakeScope,
    #[cfg(target_os = "ios")]
    previous_idle_timer_disabled: Option<bool>,
    /// A config change affects the next hold rather than the current one.
    scope: AwakeScope,
    #[cfg(not(target_os = "ios"))]
    worker: Option<std::sync::mpsc::SyncSender<desktop::Request>>,
    working: HashSet<WindowId>,
}

impl Global for Caffeine {}

pub(crate) fn cup(cx: &App) -> Cup {
    cx.try_global::<Caffeine>()
        .map_or(Cup::Off, |caffeine| caffeine.cup)
}

/// Chooses what the next hold keeps awake. Called when the config loads and
/// reloads, so a click right after a Settings change uses the new scope.
pub(crate) fn set_scope(scope: AwakeScope, cx: &mut App) {
    cx.default_global::<Caffeine>().scope = scope;
}

/// What the cup keeps awake now: the active hold's scope, or what the next
/// click would hold while the cup is off.
pub(crate) fn scope(cx: &App) -> AwakeScope {
    cx.try_global::<Caffeine>()
        .map_or_else(AwakeScope::default, |caffeine| match caffeine.cup {
            Cup::Off => caffeine.scope,
            Cup::Pending | Cup::On(_) => caffeine.held,
        })
}

/// How many holds and releases have settled so far.
#[cfg(any(test, not(target_os = "ios")))]
pub(crate) fn changes(cx: &App) -> u64 {
    cx.try_global::<Caffeine>()
        .map_or(0, |caffeine| caffeine.changes)
}

/// Forgets closed windows, so their last working agents keep nothing awake.
pub(crate) fn install(cx: &mut App) {
    if let Some(appearance) = cx.try_global::<crate::app::InitialAppearance>() {
        let scope = appearance.config.status_bar.keep_awake_scope;
        set_scope(scope, cx);
    }
    cx.on_window_closed(|cx, _| {
        let open = cx.windows();
        cx.default_global::<Caffeine>()
            .working
            .retain(|id| open.iter().any(|window| window.window_id() == *id));
        settle(cx, |error, _| {
            tracing::warn!(category = "caffeine", %error, "could not release the keep-awake hold");
        });
    })
    .detach();
}

/// Turns a hold until agents finish into one until turned off, as asking
/// for caffeine "on" means; whether it did. The hold itself stays.
#[cfg(not(target_os = "ios"))]
pub(crate) fn hold_indefinitely(cx: &mut App) -> bool {
    let caffeine = cx.default_global::<Caffeine>();
    if caffeine.cup != Cup::On(Mode::UntilAgentsFinish) {
        return false;
    }
    caffeine.cup = Cup::On(Mode::Indefinite);
    cx.refresh_windows();
    true
}

/// Records whether `window` has a working agent, and ends an
/// `UntilAgentsFinish` hold once no window does.
pub(crate) fn observe(
    window: WindowId,
    work: Work,
    cx: &mut App,
    report: impl FnOnce(crate::Error, &mut App) + 'static,
) {
    let working = &mut cx.default_global::<Caffeine>().working;
    let finished = match work {
        Work::Working => {
            working.insert(window);
            false
        }
        Work::Idle => working.remove(&window),
        // A reconnecting endpoint's agents are likely still running.
        Work::Unknown => false,
    };
    if finished {
        settle(cx, report);
    }
}

/// Ends an `UntilAgentsFinish` hold when no window has a working agent.
fn settle(cx: &mut App, report: impl FnOnce(crate::Error, &mut App) + 'static) {
    let caffeine = cx.default_global::<Caffeine>();
    if caffeine.cup == Cup::On(Mode::UntilAgentsFinish) && caffeine.working.is_empty() {
        platform::release(cx, report);
    }
}
