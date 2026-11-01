//! Desktop power assertions, held and released on one worker thread.

use super::*;
use std::sync::mpsc::{Receiver, SyncSender};

/// What the worker thread does; the mode is the app's business, not its.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Command {
    Hold,
    Release,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Change {
    Hold(Mode),
    Release,
}

impl From<Change> for Command {
    fn from(change: Change) -> Self {
        match change {
            Change::Hold(_) => Self::Hold,
            Change::Release => Self::Release,
        }
    }
}

pub(super) struct Request {
    command: Command,
    /// What a hold keeps awake, as the config said when it was asked for.
    scope: AwakeScope,
    reply: SyncSender<keepawake::Result<()>>,
}

/// Starts keeping the machine awake until the cup is clicked again, or stops
/// any hold, and redraws every window. Failures arrive later through
/// `report`, never during this call.
pub(crate) fn toggle(cx: &mut App, report: impl FnOnce(crate::Error, &mut App) + 'static) {
    toggle_with(cx, hold, report);
}

/// Starts keeping the machine awake until every agent finishes, turns an
/// indefinite hold into one, or stops one. With no agent working there is
/// nothing to wait for, which `report` hears about.
pub(crate) fn toggle_until_agents_finish(
    cx: &mut App,
    report: impl FnOnce(crate::Error, &mut App) + 'static,
) {
    toggle_until_agents_finish_with(cx, hold, report);
}

/// `start` makes the hold on the worker thread; it is used only when that
/// thread is first spawned, so tests can stand in for the real power APIs.
fn toggle_with<G>(
    cx: &mut App,
    start: impl FnMut(AwakeScope) -> keepawake::Result<G> + Send + 'static,
    report: impl FnOnce(crate::Error, &mut App) + 'static,
) {
    match cup(cx) {
        Cup::Pending => {}
        Cup::Off => hold_with(cx, Mode::Indefinite, start, report),
        Cup::On(_) => release(cx, report),
    }
}

fn toggle_until_agents_finish_with<G>(
    cx: &mut App,
    start: impl FnMut(AwakeScope) -> keepawake::Result<G> + Send + 'static,
    report: impl FnOnce(crate::Error, &mut App) + 'static,
) {
    let caffeine = cx.default_global::<Caffeine>();
    let idle = caffeine.working.is_empty();
    match caffeine.cup {
        Cup::Pending => {}
        Cup::On(Mode::UntilAgentsFinish) => release(cx, report),
        _ if idle => cx.defer(move |cx| report(Error::NoWorkingAgents.into(), cx)),
        Cup::On(Mode::Indefinite) => {
            caffeine.cup = Cup::On(Mode::UntilAgentsFinish);
            cx.refresh_windows();
        }
        Cup::Off => hold_with(cx, Mode::UntilAgentsFinish, start, report),
    }
}

fn hold_with<G>(
    cx: &mut App,
    mode: Mode,
    start: impl FnMut(AwakeScope) -> keepawake::Result<G> + Send + 'static,
    report: impl FnOnce(crate::Error, &mut App) + 'static,
) {
    let worker = match cx.default_global::<Caffeine>().worker.take() {
        Some(worker) => Ok(worker),
        None => spawn(start),
    };
    send(cx, Change::Hold(mode), worker, report);
}

pub(super) fn release(cx: &mut App, report: impl FnOnce(crate::Error, &mut App) + 'static) {
    let caffeine = cx.default_global::<Caffeine>();
    let Some(worker) = caffeine.worker.take() else {
        // No worker holds nothing.
        caffeine.cup = Cup::Off;
        cx.refresh_windows();
        return;
    };
    send(cx, Change::Release, Ok(worker), report);
}

fn send(
    cx: &mut App,
    change: Change,
    worker: Result<SyncSender<Request>>,
    report: impl FnOnce(crate::Error, &mut App) + 'static,
) {
    let (reply, replies) = std::sync::mpsc::sync_channel(1);
    let scope = cx.default_global::<Caffeine>().scope;
    let sent = worker.and_then(|worker| {
        // Never blocks: `Pending` keeps at most one request in the slot.
        worker
            .send(Request {
                command: change.into(),
                scope,
                reply,
            })
            .map(|()| worker)
            .map_err(|_| crate::Error::from(Error::WorkerStopped))
    });
    let caffeine = cx.default_global::<Caffeine>();
    match sent {
        Ok(worker) => caffeine.worker = Some(worker),
        Err(error) => {
            cx.defer(move |cx| report(error, cx));
            return;
        }
    }
    caffeine.cup = Cup::Pending;
    if matches!(change, Change::Hold(_)) {
        caffeine.held = scope;
    }
    cx.refresh_windows();
    let wait = cx
        .background_executor()
        .spawn(async move { replies.recv() });
    cx.spawn(async move |cx| {
        // A dropped reply means the worker died, e.g. a panicking release.
        let result = match wait.await {
            Ok(result) => result.map_err(|error| Error::Hold(error).into()),
            Err(_) => Err(Error::WorkerStopped.into()),
        };
        cx.update(|cx| finish(cx, change, result, report));
    })
    .detach();
}

fn finish(
    cx: &mut App,
    change: Change,
    result: Result<()>,
    report: impl FnOnce(crate::Error, &mut App) + 'static,
) {
    let caffeine = cx.default_global::<Caffeine>();
    caffeine.cup = match (change, &result) {
        (Change::Hold(mode), Ok(())) => Cup::On(mode),
        // A failed hold holds nothing, and a worker that died mid-release
        // took its guard with it.
        (Change::Hold(_), Err(_)) | (Change::Release, _) => Cup::Off,
    };
    {
        caffeine.changes = caffeine.changes.wrapping_add(1);
    }
    if matches!(result, Err(crate::Error::Caffeine(Error::WorkerStopped))) {
        caffeine.worker = None;
    }
    cx.refresh_windows();
    match result {
        Err(error) => report(error, cx),
        // The agents may have finished while the hold was being made.
        Ok(()) => settle(cx, report),
    }
}

fn spawn<G>(
    start: impl FnMut(AwakeScope) -> keepawake::Result<G> + Send + 'static,
) -> Result<SyncSender<Request>> {
    let (worker, requests) = std::sync::mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("herdr-keepawake".into())
        .spawn(move || run(&requests, start))
        .map_err(Error::Thread)?;
    Ok(worker)
}

/// Serves requests until the app drops its sender, then drops any hold here.
fn run<G>(requests: &Receiver<Request>, mut start: impl FnMut(AwakeScope) -> keepawake::Result<G>) {
    let mut guard = None;
    for Request {
        command,
        scope,
        reply,
    } in requests
    {
        let result = match command {
            Command::Hold if guard.is_some() => Ok(()),
            Command::Hold => start(scope).map(|held| guard = Some(held)),
            Command::Release => {
                guard = None;
                Ok(())
            }
        };
        // The waiting task outlives the request unless the app is quitting.
        let _ = reply.send(result);
    }
}

fn hold(scope: AwakeScope) -> keepawake::Result<keepawake::KeepAwake> {
    // Idle, not `sleep`: that one needs AC power and does nothing under
    // Windows Modern Standby, and the lid closing should still sleep. The
    // display is held too unless the scope lets it sleep and lock.
    keepawake::Builder::default()
        .display(scope == AwakeScope::Display)
        .idle(true)
        .reason(match scope {
            AwakeScope::Display => "Herdr keeps the display awake",
            AwakeScope::System => "Herdr keeps the system awake",
        })
        .app_name("Herdr")
        .app_reverse_domain(crate::constants::APP_ID)
        .create()
}

#[cfg(test)]
mod tests;
