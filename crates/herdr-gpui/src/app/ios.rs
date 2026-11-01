//! The UIKit entry point uses the same windows and presentation as desktop.
//! Services that need a local daemon or child processes are never started here.

use super::{Config, InitialAppearance, StartupAppearance, light_appearance, open_window};
use crate::{APP_VERSION, bind_keys, diagnostics, icons, updater};
use gpui::{App, Application};
use herdr_client::ConnectTarget;

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
    let failed = std::rc::Rc::new(std::cell::Cell::new(false));
    let outcome = failed.clone();
    let application: Application = herdr_uikit::application().with_assets(icons::Icons);
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
                tracing::error!(%error, "Unable to open the iPad window");
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
