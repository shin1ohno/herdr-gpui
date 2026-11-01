//! The UIKit entry point uses the same windows and presentation as desktop.
//! Services that need a local daemon or child processes are never started here.

use super::{Config, InitialAppearance, StartupAppearance, light_appearance, open_window};
use crate::{APP_VERSION, bind_keys, diagnostics, icons, state_file, updater};
use gpui::{App, AppLifecyclePhase, Application, AsyncApp, Platform};
use herdr_client::ConnectTarget;
use herdr_uikit::{BackgroundTask, IosPlatform};
use std::{rc::Rc, time::Duration};

/// How long saving state may take once the app is in the background. The
/// system grants a background task about 30 seconds; every store writes one
/// small file.
const BACKGROUND_FLUSH_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) fn run() -> std::process::ExitCode {
    if let Err(error) = diagnostics::init(crate::log_targets::LOG_TARGETS) {
        eprintln!("Unable to initialize diagnostics: {error}");
        return std::process::ExitCode::FAILURE;
    }
    crate::crash::install(APP_VERSION);
    let appearance = StartupAppearance::load(Config::load_startup);
    let browser_tabs = crate::browser::Store::load();
    let group_layouts = crate::browser::Layouts::load();
    let worktree_notes = crate::worktree_notes::Notes::load();
    let review_viewed = crate::review::Viewed::load();
    let overview_marks = crate::overview::Marks::load();
    let notification_mutes = crate::notification_mutes::Mutes::load();
    let palette_frecency = crate::palette::Frecency::load();
    let dispatch_history = crate::dispatch::History::load();
    let fan_out_journal = crate::fan_out::Journal::load();
    let keychain_grants = crate::usage::KeychainGrants::load();
    let window_state = crate::window_state::WindowState::load();
    let failed = Rc::new(std::cell::Cell::new(false));
    let outcome = failed.clone();
    let platform = Rc::new(IosPlatform::new());
    let application = Application::with_platform(platform.clone()).with_assets(icons::Icons);
    let urls_cx = application.to_async();
    application.on_open_urls(move |urls| {
        urls_cx.update(|cx| crate::host_key_code::open_urls(urls, cx));
    });
    application.run(move |cx| {
        browser_tabs.install(cx);
        group_layouts.install(cx);
        worktree_notes.install(cx);
        review_viewed.install(cx);
        overview_marks.install(cx);
        notification_mutes.install(cx);
        palette_frecency.install(cx);
        dispatch_history.install(cx);
        // These stores restore client-local presentation/history only. Their
        // constructors do not dispatch work, read sign-ins, or start a daemon.
        fan_out_journal.install(cx);
        keychain_grants.install(cx);
        window_state.install(cx);
        // After the stores, so a flush always has them to write.
        watch_lifecycle(&platform, cx);
        cx.set_global(appearance.select(light_appearance(cx)));
        let language = cx.global::<InitialAppearance>().config.language.clone();
        crate::i18n::install(&language, cx);
        crate::caffeine::install(cx);
        crate::endpoint::watch_network();
        // Both launchers retain address/settings state on platforms without
        // embedded pages. Their EMBEDDED guard prevents probing or starting a tool.
        crate::code_server::Launcher::install(cx);
        crate::px0::Launcher::install(cx);
        bind_keys(cx);
        install_actions(cx);
        // Endpoint zero remains the window's empty selection until a saved
        // device connects. It is disabled and never dials a local socket on iOS.
        match open_window(
            ConnectTarget::Local,
            updater::Updater::default(),
            cx,
            #[cfg(feature = "integration-test")]
            false,
        ) {
            Ok(window) => {
                let sound = super::shared_sound(cx);
                let _ = window.update(cx, |view, _, _| view.use_sound(sound));
            }
            Err(error) => {
                // The alternate form keeps anyhow's cause chain in the log.
                tracing::error!(error = %format_args!("{error:#}"), "Unable to open the iPad window");
                outcome.set(true);
                cx.quit();
            }
        }
        cx.activate(true);
    });
    if failed.get() {
        std::process::ExitCode::FAILURE
    } else {
        std::process::ExitCode::SUCCESS
    }
}

/// Saves state whenever the app goes to the background, since iPadOS may end
/// it there without a quit. The stores' quit hooks stop their writers, so
/// they are not used; everything keeps running for when the app returns.
fn watch_lifecycle(platform: &IosPlatform, cx: &App) {
    let cx = cx.to_async();
    platform.on_app_lifecycle(Box::new(move |phase| {
        if phase == AppLifecyclePhase::Background {
            flush_for_background(&cx);
        }
    }));
}

/// The platform calls lifecycle handlers from their own main-queue block,
/// outside any app update, so updating the app here cannot borrow it twice.
fn flush_for_background(cx: &AsyncApp) {
    // Held until the Settings drafts and the stores are written, or the
    // flush's deadline passes: the platform's own task ends when this handler
    // returns.
    let task = BackgroundTask::begin("Saving state");
    let flush = cx.update(|cx| state_file::background::flush(cx, BACKGROUND_FLUSH_TIMEOUT));
    cx.foreground_executor()
        .spawn(async move {
            let flushed = flush.await;
            drop(task);
            tracing::info!(
                category = "lifecycle",
                drafts = flushed.drafts,
                synced = flushed.stores.synced,
                lost = flushed.stores.lost,
                timed_out = flushed.stores.timed_out,
                "Saved state for the background"
            );
        })
        .detach();
}

fn install_actions(cx: &mut App) {
    cx.on_action(|_: &crate::Quit, cx| cx.quit());
    cx.on_action(|_: &crate::ShowLogs, cx| crate::log_window::open(cx));
    cx.on_action(|action: &crate::actions::TogglePseudoLocale, cx| {
        crate::i18n::toggle_pseudo(action.pseudo, cx);
    });
    cx.on_action(|_: &crate::actions::ToggleSlowAnimations, cx| {
        crate::motion::set_slow(!crate::motion::is_slow());
        cx.refresh_windows();
    });
}
